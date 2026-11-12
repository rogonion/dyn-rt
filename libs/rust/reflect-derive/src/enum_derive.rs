//! `#[derive(ReflectEnum)]` expansion: base `Reflect` impl (routing `Enum`)
//! + the `ReflectEnum` shape impl.
//!
//! Supports unit variants, single-payload tuple variants, and struct
//! variants (`Variant { field: .. }`).

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Fields, Meta};

use crate::attrs::{lit_str, parse_field_attrs, parse_type_attrs};
use crate::base::{base_impl, new_impl};

/// One named field of a struct variant.
struct VariantField {
    ident: syn::Ident,
    external: String,
    ty: syn::Type,
}

/// The shape of one enum variant.
enum VariantShape {
    Unit,
    Tuple(syn::Type),
    Struct(Vec<VariantField>),
}

pub fn expand(input: DeriveInput) -> TokenStream {
    let attrs = match parse_type_attrs(&input.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "ReflectEnum currently requires a non-generic type",
        )
        .to_compile_error()
        .into();
    }
    let crate_path = &attrs.crate_path;
    let name = &input.ident;
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return syn::Error::new_spanned(name, "ReflectEnum can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    // Per variant: ident, external name, shape.
    let mut v_idents: Vec<syn::Ident> = Vec::new();
    let mut v_names: Vec<String> = Vec::new();
    let mut v_shapes: Vec<VariantShape> = Vec::new();
    for variant in variants {
        let mut rename: Option<String> = None;
        for attr in &variant.attrs {
            if !attr.path().is_ident("reflect") {
                continue;
            }
            let metas =
                match attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated) {
                    Ok(metas) => metas,
                    Err(err) => return err.to_compile_error().into(),
                };
            for meta in metas {
                match meta {
                    Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                        match lit_str(&nv, "rename") {
                            Ok(lit) => rename = Some(lit.value()),
                            Err(err) => return err.to_compile_error().into(),
                        }
                    }
                    other => {
                        return syn::Error::new_spanned(
                            other,
                            "unsupported #[reflect(...)] variant attribute",
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
        let shape = match &variant.fields {
            Fields::Unit => VariantShape::Unit,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                VariantShape::Tuple(unnamed.unnamed.first().unwrap().ty.clone())
            }
            Fields::Named(named) => {
                let mut fields: Vec<VariantField> = Vec::new();
                for field in &named.named {
                    let fattrs = match parse_field_attrs(&field.attrs) {
                        Ok(fattrs) => fattrs,
                        Err(err) => return err.to_compile_error().into(),
                    };
                    if fattrs.skip {
                        return syn::Error::new_spanned(
                            field,
                            "skip is not supported on enum struct-variant fields",
                        )
                        .to_compile_error()
                        .into();
                    }
                    let ident = field.ident.clone().expect("named field");
                    let external = fattrs.rename.unwrap_or_else(|| ident.to_string());
                    fields.push(VariantField {
                        ident,
                        external,
                        ty: field.ty.clone(),
                    });
                }
                VariantShape::Struct(fields)
            }
            other => {
                return syn::Error::new_spanned(
                    other,
                    "ReflectEnum supports unit variants, single-payload tuple variants, and struct variants only",
                )
                .to_compile_error()
                .into();
            }
        };
        v_idents.push(variant.ident.clone());
        v_names.push(rename.unwrap_or_else(|| variant.ident.to_string()));
        v_shapes.push(shape);
    }

    let has_non_struct = v_shapes
        .iter()
        .any(|s| !matches!(s, VariantShape::Struct(_)));

    // active variant name
    let active_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_names)
        .zip(&v_shapes)
        .map(|((ident, name), shape)| match shape {
            VariantShape::Unit => quote!(Self::#ident => #name,),
            VariantShape::Tuple(_) => quote!(Self::#ident(_) => #name,),
            VariantShape::Struct(_) => quote!(Self::#ident { .. } => #name,),
        })
        .collect();

    // payload access: only tuple variants have a single payload; struct
    // variants expose their data through the field_* methods instead
    let payload_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .map(|(ident, shape)| match shape {
            VariantShape::Tuple(_) => {
                quote!(Self::#ident(payload) => ::core::option::Option::Some(payload as &dyn #crate_path::Reflect),)
            }
            VariantShape::Unit => quote!(Self::#ident => ::core::option::Option::None,),
            VariantShape::Struct(_) => quote!(Self::#ident { .. } => ::core::option::Option::None,),
        })
        .collect();
    let payload_mut_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .map(|(ident, shape)| match shape {
            VariantShape::Tuple(_) => {
                quote!(Self::#ident(payload) => ::core::option::Option::Some(payload as &mut dyn #crate_path::Reflect),)
            }
            VariantShape::Unit => quote!(Self::#ident => ::core::option::Option::None,),
            VariantShape::Struct(_) => {
                quote!(Self::#ident { .. } => ::core::option::Option::None,)
            }
        })
        .collect();

    // active-variant field table: one static per struct variant, each in its
    // own block scope (the impl is non-generic, enforced above)
    let field_infos_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .filter_map(|(ident, shape)| match shape {
            VariantShape::Struct(fields) => {
                let exts: Vec<&String> = fields.iter().map(|f| &f.external).collect();
                let tys: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();
                Some(quote! {
                    Self::#ident { .. } => {
                        static FIELDS: ::std::sync::OnceLock<::std::vec::Vec<#crate_path::FieldInfo>> =
                            ::std::sync::OnceLock::new();
                        FIELDS
                            .get_or_init(|| ::std::vec![
                                #(
                                    #crate_path::FieldInfo {
                                        name: #exts,
                                        type_info: #crate_path::type_info::<#tys>(),
                                    },
                                )*
                            ])
                            .as_slice()
                    }
                })
            }
            _ => None,
        })
        .collect();
    let field_infos_fallback = if has_non_struct {
        Some(quote!(_ => &[],))
    } else {
        None
    };

    // active-variant field access by index
    let field_at_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .filter_map(|(ident, shape)| match shape {
            VariantShape::Struct(fields) => {
                let fidents: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();
                let indices: Vec<usize> = (0..fields.len()).collect();
                Some(quote! {
                    Self::#ident { #(#fidents),* } => match index {
                        #(#indices => ::core::option::Option::Some(#fidents as &dyn #crate_path::Reflect),)*
                        _ => ::core::option::Option::None,
                    },
                })
            }
            _ => None,
        })
        .collect();
    let field_at_fallback = if has_non_struct {
        Some(quote!(_ => ::core::option::Option::None,))
    } else {
        None
    };
    let field_at_mut_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .filter_map(|(ident, shape)| match shape {
            VariantShape::Struct(fields) => {
                let fidents: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();
                let indices: Vec<usize> = (0..fields.len()).collect();
                Some(quote! {
                    Self::#ident { #(#fidents),* } => match index {
                        #(#indices => ::core::option::Option::Some(#fidents as &mut dyn #crate_path::Reflect),)*
                        _ => ::core::option::Option::None,
                    },
                })
            }
            _ => None,
        })
        .collect();
    let field_at_mut_fallback = if has_non_struct {
        Some(quote!(_ => ::core::option::Option::None,))
    } else {
        None
    };

    // field table per variant name (static introspection, e.g. for schema)
    let variant_field_infos_arms: Vec<_> = v_names
        .iter()
        .zip(&v_shapes)
        .filter_map(|(name, shape)| match shape {
            VariantShape::Struct(fields) => {
                let exts: Vec<&String> = fields.iter().map(|f| &f.external).collect();
                let tys: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();
                Some(quote! {
                    #name => {
                        static FIELDS: ::std::sync::OnceLock<::std::vec::Vec<#crate_path::FieldInfo>> =
                            ::std::sync::OnceLock::new();
                        FIELDS
                            .get_or_init(|| ::std::vec![
                                #(
                                    #crate_path::FieldInfo {
                                        name: #exts,
                                        type_info: #crate_path::type_info::<#tys>(),
                                    },
                                )*
                            ])
                            .as_slice()
                    }
                })
            }
            _ => None,
        })
        .collect();

    // set_variant arms
    let set_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_names)
        .zip(&v_shapes)
        .map(|((ident, name), shape)| match shape {
            VariantShape::Unit => quote! {
                #name => {
                    if payload.is_some() {
                        return ::core::result::Result::Err(#crate_path::ReflectError::ShapeMismatch {
                            operation: "reflect_set_variant",
                            target: ::core::any::type_name::<Self>().to_string(),
                            source: "unit variant takes no payload".to_string(),
                        });
                    }
                    *self = Self::#ident;
                    ::core::result::Result::Ok(())
                }
            },
            VariantShape::Tuple(ty) => quote! {
                #name => {
                    let payload = payload.ok_or_else(|| #crate_path::ReflectError::UnknownVariant {
                        variant: ::std::format!("{} (missing payload)", #name),
                        type_name: ::core::any::type_name::<Self>().to_string(),
                    })?;
                    let typed = payload
                        .reflect_into_any()
                        .downcast::<#ty>()
                        .map_err(|_| #crate_path::ReflectError::ShapeMismatch {
                            operation: "reflect_set_variant",
                            target: ::core::any::type_name::<Self>().to_string(),
                            source: "payload type mismatch".to_string(),
                        })?;
                    *self = Self::#ident(*typed);
                    ::core::result::Result::Ok(())
                }
            },
            VariantShape::Struct(fields) => {
                let fidents: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();
                let exts: Vec<&String> = fields.iter().map(|f| &f.external).collect();
                let tys: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();
                quote! {
                    #name => {
                        let payload = payload.ok_or_else(|| #crate_path::ReflectError::UnknownVariant {
                            variant: ::std::format!("{} (missing fields payload)", #name),
                            type_name: ::core::any::type_name::<Self>().to_string(),
                        })?;
                        let source_fields = #crate_path::as_struct(&*payload).ok_or_else(|| #crate_path::ReflectError::ShapeMismatch {
                            operation: "reflect_set_variant",
                            target: ::core::any::type_name::<Self>().to_string(),
                            source: "struct variant payload must be a struct".to_string(),
                        })?;
                        *self = Self::#ident {
                            #(
                                #fidents: {
                                    let field = source_fields.reflect_field(#exts).ok_or_else(|| #crate_path::ReflectError::ShapeMismatch {
                                        operation: "reflect_set_variant",
                                        target: ::core::any::type_name::<Self>().to_string(),
                                        source: ::std::format!("payload has no field {}", #exts),
                                    })?;
                                    *field.reflect_clone().reflect_into_any().downcast::<#tys>().map_err(|_| #crate_path::ReflectError::ShapeMismatch {
                                        operation: "reflect_set_variant",
                                        target: ::core::any::type_name::<Self>().to_string(),
                                        source: ::std::format!("payload field {} has the wrong type", #exts),
                                    })?
                                },
                            )*
                        };
                        ::core::result::Result::Ok(())
                    }
                }
            }
        })
        .collect();

    // clone arms
    let clone_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .map(|(ident, shape)| match shape {
            VariantShape::Unit => quote!(Self::#ident => ::std::boxed::Box::new(Self::#ident),),
            VariantShape::Tuple(ty) => quote! {
                Self::#ident(payload) => ::std::boxed::Box::new(Self::#ident(
                    *payload.reflect_clone().reflect_into_any().downcast::<#ty>().expect("payload clone returns the payload type")
                )),
            },
            VariantShape::Struct(fields) => {
                let fidents: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();
                let tys: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();
                quote! {
                    Self::#ident { #(#fidents),* } => ::std::boxed::Box::new(Self::#ident {
                        #(
                            #fidents: *#fidents.reflect_clone().reflect_into_any().downcast::<#tys>().expect("field clone returns the field type"),
                        )*
                    }),
                }
            }
        })
        .collect();

    // apply arms (same variant on both sides; fields/payload applied structurally)
    let apply_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_shapes)
        .map(|(ident, shape)| match shape {
            VariantShape::Unit => quote!((Self::#ident, Self::#ident) => ::core::result::Result::Ok(()),),
            VariantShape::Tuple(_) => quote! {
                (Self::#ident(target), Self::#ident(source)) => {
                    target.reflect_apply(source)?;
                    ::core::result::Result::Ok(())
                }
            },
            VariantShape::Struct(fields) => {
                let fidents: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();
                let src_fidents: Vec<syn::Ident> = fields
                    .iter()
                    .map(|f| format_ident!("src_{}", f.ident))
                    .collect();
                quote! {
                    (Self::#ident { #(#fidents),* }, Self::#ident { #(#fidents: #src_fidents),* }) => {
                        #(#fidents.reflect_apply(#src_fidents)?;)*
                        ::core::result::Result::Ok(())
                    }
                }
            }
        })
        .collect();

    let clone_body = quote! { match self { #(#clone_arms)* } };
    let apply_body = quote! {
        {
            let source = value
                .reflect_as_any()
                .downcast_ref::<Self>()
                .ok_or_else(|| #crate_path::ReflectError::ShapeMismatch {
                    operation: "reflect_apply",
                    target: ::core::any::type_name::<Self>().to_string(),
                    source: value.reflect_type_name().to_string(),
                })?;
            match (self, source) {
                #(#apply_arms)*
                _ => ::core::result::Result::Err(#crate_path::ReflectError::ShapeMismatch {
                    operation: "reflect_apply",
                    target: ::core::any::type_name::<Self>().to_string(),
                    source: ::std::format!("active variant {}", source.reflect_active_variant()),
                }),
            }
        }
    };

    let variant_infos: Vec<_> = v_names
        .iter()
        .zip(&v_shapes)
        .map(|(name, shape)| {
            let payload_info = match shape {
                VariantShape::Tuple(ty) => {
                    quote!(::core::option::Option::Some(#crate_path::type_info::<#ty>()))
                }
                _ => quote!(::core::option::Option::None),
            };
            quote! {
                #crate_path::VariantInfo {
                    name: #name,
                    payload_type_info: #payload_info,
                }
            }
        })
        .collect();

    let shape_impl = quote! {
        impl #crate_path::ReflectEnum for #name {
            fn reflect_active_variant(&self) -> &'static str {
                match self {
                    #(#active_arms)*
                }
            }

            fn reflect_variants(&self) -> &'static [#crate_path::VariantInfo] {
                // OnceLock because type_info() is not const; the impl is
                // non-generic (enforced above), so a plain static is per-type.
                static VARIANTS: ::std::sync::OnceLock<::std::vec::Vec<#crate_path::VariantInfo>> =
                    ::std::sync::OnceLock::new();
                VARIANTS
                    .get_or_init(|| ::std::vec![#(#variant_infos),*])
                    .as_slice()
            }

            fn reflect_active_variant_payload(&self) -> ::core::option::Option<&dyn #crate_path::Reflect> {
                match self {
                    #(#payload_arms)*
                }
            }

            fn reflect_active_variant_payload_mut(&mut self) -> ::core::option::Option<&mut dyn #crate_path::Reflect> {
                match self {
                    #(#payload_mut_arms)*
                }
            }

            fn reflect_active_variant_field_infos(&self) -> &'static [#crate_path::FieldInfo] {
                match self {
                    #(#field_infos_arms)*
                    #field_infos_fallback
                }
            }

            fn reflect_active_variant_field_at(&self, index: usize) -> ::core::option::Option<&dyn #crate_path::Reflect> {
                match self {
                    #(#field_at_arms)*
                    #field_at_fallback
                }
            }

            fn reflect_active_variant_field_at_mut(&mut self, index: usize) -> ::core::option::Option<&mut dyn #crate_path::Reflect> {
                match self {
                    #(#field_at_mut_arms)*
                    #field_at_mut_fallback
                }
            }

            fn reflect_variant_field_infos(&self, variant_name: &str) -> &'static [#crate_path::FieldInfo] {
                match variant_name {
                    #(#variant_field_infos_arms)*
                    _ => &[],
                }
            }

            fn reflect_set_variant(
                &mut self,
                variant_name: &str,
                payload: ::core::option::Option<::std::boxed::Box<dyn #crate_path::Reflect>>,
            ) -> ::core::result::Result<(), #crate_path::ReflectError> {
                match variant_name {
                    #(#set_arms)*
                    other => ::core::result::Result::Err(#crate_path::ReflectError::UnknownVariant {
                        variant: other.to_string(),
                        type_name: ::core::any::type_name::<Self>().to_string(),
                    }),
                }
            }
        }
    };

    let mut out = base_impl(
        &input,
        &attrs,
        syn::Ident::new("Enum", proc_macro2::Span::call_site()),
        clone_body,
        apply_body,
        false,
    );
    out.extend(shape_impl);
    if attrs.new {
        out.extend(new_impl(&input, &attrs));
    }
    out.into()
}
