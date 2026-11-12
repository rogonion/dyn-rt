//! `#[derive(ReflectEnum)]` expansion: base `Reflect` impl (routing `Enum`)
//! + the `ReflectEnum` shape impl.

use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Fields, Meta};

use crate::attrs::{lit_str, parse_type_attrs};
use crate::base::{base_impl, new_impl};

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

    // Per variant: ident, external name, payload type (unit or single tuple).
    let mut v_idents: Vec<syn::Ident> = Vec::new();
    let mut v_names: Vec<String> = Vec::new();
    let mut v_payload_tys: Vec<Option<syn::Type>> = Vec::new();
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
        let payload_ty = match &variant.fields {
            Fields::Unit => None,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                Some(unnamed.unnamed.first().unwrap().ty.clone())
            }
            other => {
                return syn::Error::new_spanned(
                    other,
                    "ReflectEnum supports unit variants and single-payload tuple variants only",
                )
                .to_compile_error()
                .into();
            }
        };
        v_idents.push(variant.ident.clone());
        v_names.push(rename.unwrap_or_else(|| variant.ident.to_string()));
        v_payload_tys.push(payload_ty);
    }

    // active variant name
    let active_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_names)
        .zip(&v_payload_tys)
        .map(|((ident, name), payload)| match payload {
            Some(_) => quote!(Self::#ident(_) => #name,),
            None => quote!(Self::#ident => #name,),
        })
        .collect();

    // payload access
    let payload_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_payload_tys)
        .map(|(ident, payload)| match payload {
            Some(_) => quote!(Self::#ident(payload) => ::core::option::Option::Some(payload as &dyn #crate_path::Reflect),),
            None => quote!(Self::#ident => ::core::option::Option::None,),
        })
        .collect();
    let payload_mut_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_payload_tys)
        .map(|(ident, payload)| match payload {
            Some(_) => quote!(Self::#ident(payload) => ::core::option::Option::Some(payload as &mut dyn #crate_path::Reflect),),
            None => quote!(Self::#ident => ::core::option::Option::None,),
        })
        .collect();

    // set_variant arms
    let set_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_names)
        .zip(&v_payload_tys)
        .map(|((ident, name), payload)| match payload {
            None => quote! {
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
            Some(ty) => quote! {
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
        })
        .collect();

    // clone arms
    let clone_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_payload_tys)
        .map(|(ident, payload)| match payload {
            None => quote!(Self::#ident => ::std::boxed::Box::new(Self::#ident),),
            Some(ty) => quote! {
                Self::#ident(payload) => ::std::boxed::Box::new(Self::#ident(
                    *payload.reflect_clone().reflect_into_any().downcast::<#ty>().expect("payload clone returns the payload type")
                )),
            },
        })
        .collect();

    // apply arms (same variant on both sides; payload applied structurally)
    let apply_arms: Vec<_> = v_idents
        .iter()
        .zip(&v_payload_tys)
        .map(|(ident, payload)| match payload {
            None => quote!((Self::#ident, Self::#ident) => ::core::result::Result::Ok(()),),
            Some(_) => quote! {
                (Self::#ident(target), Self::#ident(source)) => {
                    target.reflect_apply(source)?;
                    ::core::result::Result::Ok(())
                }
            },
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
        .zip(&v_payload_tys)
        .map(|(name, payload)| {
            let payload_info = match payload {
                Some(ty) => quote!(::core::option::Option::Some(#crate_path::type_info::<#ty>())),
                None => quote!(::core::option::Option::None),
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
