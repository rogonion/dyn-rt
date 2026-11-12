//! Derive macros for `dyn-rt-reflect` (ADR 0008). Per-shape, each derive emits
//! the base `Reflect` impl as well, so one derive per type:
//!
//! - `#[derive(Reflect)]` — base trait only, routes to `Unknown`; requires
//!   `Clone` (clone/apply are clone-based).
//! - `#[derive(ReflectStruct)]` — structs (non-generic for now); structural
//!   clone/apply field-by-field, so fields must implement `Reflect`.
//! - `#[derive(ReflectEnum)]` — enums with unit or single-payload tuple
//!   variants; structural clone/apply per payload.
//!
//! Attributes (all under `#[reflect(...)]`):
//! - type level: `crate = "path"` (reflect crate path, default `dyn_rt_reflect`),
//!   `name = "..."` (override reflect_type_name), `new` (emit ReflectNew via Default)
//! - struct fields: `rename = "..."`, `skip` (skipped fields clone via `Clone`)
//! - enum variants: `rename = "..."`

use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, Meta, parse_macro_input, parse_quote};

// ---------------------------------------------------------------------------
// attribute parsing
// ---------------------------------------------------------------------------

struct TypeAttrs {
    crate_path: syn::Path,
    name: Option<syn::LitStr>,
    new: bool,
}

fn lit_str(nv: &syn::MetaNameValue, what: &str) -> syn::Result<syn::LitStr> {
    match &nv.value {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Str(lit) => Ok(lit.clone()),
            other => Err(syn::Error::new_spanned(
                other,
                format!("{what} must be a string literal"),
            )),
        },
        other => Err(syn::Error::new_spanned(
            other,
            format!("{what} must be a string literal"),
        )),
    }
}

fn parse_type_attrs(attrs: &[Attribute]) -> syn::Result<TypeAttrs> {
    let mut out = TypeAttrs {
        crate_path: parse_quote!(dyn_rt_reflect),
        name: None,
        new: false,
    };
    for attr in attrs {
        if !attr.path().is_ident("reflect") {
            continue;
        }
        let metas = attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;
        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("crate") => {
                    out.crate_path = lit_str(&nv, "crate")?.parse()?;
                }
                Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    out.name = Some(lit_str(&nv, "name")?);
                }
                Meta::Path(p) if p.is_ident("new") => out.new = true,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unsupported #[reflect(...)] attribute",
                    ));
                }
            }
        }
    }
    Ok(out)
}

struct FieldAttrs {
    rename: Option<String>,
    skip: bool,
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut out = FieldAttrs {
        rename: None,
        skip: false,
    };
    for attr in attrs {
        if !attr.path().is_ident("reflect") {
            continue;
        }
        let metas = attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;
        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                    out.rename = Some(lit_str(&nv, "rename")?.value());
                }
                Meta::Path(p) if p.is_ident("skip") => out.skip = true,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unsupported #[reflect(...)] field attribute",
                    ));
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// base Reflect impl generation (shared by all three derives)
// ---------------------------------------------------------------------------

/// Generate the base `Reflect` impl. `route` is `Unknown`, `Struct` or `Enum`;
/// `clone_body` / `apply_body` are the per-shape bodies; `require_clone` adds a
/// `Self: Clone` where-predicate (base derive only).
fn base_impl(
    input: &DeriveInput,
    attrs: &TypeAttrs,
    route: syn::Ident,
    clone_body: proc_macro2::TokenStream,
    apply_body: proc_macro2::TokenStream,
    require_clone: bool,
) -> proc_macro2::TokenStream {
    let crate_path = &attrs.crate_path;
    let name = &input.ident;
    let mut generics = input.generics.clone();
    if require_clone {
        let ty_generics = input.generics.split_for_impl().1;
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#name #ty_generics: ::core::clone::Clone));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let type_name = match &attrs.name {
        Some(lit) => quote!(#lit),
        None => quote!(::core::any::type_name::<Self>()),
    };
    let kind = &route;
    quote! {
        impl #impl_generics #crate_path::Reflect for #name #ty_generics #where_clause {
            fn reflect_type_name(&self) -> &'static str {
                #type_name
            }

            fn reflect_ref(&self) -> #crate_path::ReflectRef<'_> {
                #crate_path::ReflectRef::#route(self)
            }

            fn reflect_ref_mut(&mut self) -> #crate_path::ReflectRefMut<'_> {
                #crate_path::ReflectRefMut::#route(self)
            }

            fn reflect_clone(&self) -> ::std::boxed::Box<dyn #crate_path::Reflect> {
                #clone_body
            }

            fn reflect_apply(&mut self, value: &dyn #crate_path::Reflect)
                -> ::core::result::Result<(), #crate_path::ReflectError>
            {
                #apply_body
            }

            fn reflect_as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn reflect_as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn reflect_into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::core::any::Any> {
                self
            }

            fn reflect_kind() -> #crate_path::ReflectKind {
                #crate_path::ReflectKind::#kind
            }
        }
    }
}

fn new_impl(input: &DeriveInput, attrs: &TypeAttrs) -> proc_macro2::TokenStream {
    let crate_path = &attrs.crate_path;
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics #crate_path::ReflectNew for #name #ty_generics #where_clause {
            fn reflect_new() -> ::std::boxed::Box<dyn #crate_path::Reflect> {
                ::std::boxed::Box::new(<Self as ::core::default::Default>::default())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// #[derive(Reflect)] — base only, routes to Unknown, Clone-based
// ---------------------------------------------------------------------------

#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let attrs = match parse_type_attrs(&input.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let crate_path = &attrs.crate_path;
    let clone_body = quote! { ::std::boxed::Box::new(self.clone()) };
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
            *self = source.clone();
            ::core::result::Result::Ok(())
        }
    };
    let mut out = base_impl(
        &input,
        &attrs,
        syn::Ident::new("Unknown", proc_macro2::Span::call_site()),
        clone_body,
        apply_body,
        true,
    );
    if attrs.new {
        out.extend(new_impl(&input, &attrs));
    }
    out.into()
}

// ---------------------------------------------------------------------------
// #[derive(ReflectStruct)]
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// #[derive(ReflectStruct)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(ReflectStruct, attributes(reflect))]
pub fn derive_reflect_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let attrs = match parse_type_attrs(&input.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    // The field table is a `static` in the generated fn; statics in generic
    // fns are shared across monomorphizations, so restrict to non-generic.
    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "ReflectStruct currently requires a non-generic type",
        )
        .to_compile_error()
        .into();
    }
    let crate_path = &attrs.crate_path;
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return syn::Error::new_spanned(name, "ReflectStruct can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };
    let named = matches!(fields, Fields::Named(_));

    // One pass over the fields. Every field gets a clone expr and an apply
    // stmt (skipped fields fall back to `Clone`); non-skipped fields also get
    // a field-table entry (accessor, external name, type).
    let mut table_accessors: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut table_names: Vec<String> = Vec::new();
    let mut table_tys: Vec<syn::Type> = Vec::new();
    let mut clone_parts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut apply_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let fattrs = match parse_field_attrs(&field.attrs) {
            Ok(fattrs) => fattrs,
            Err(err) => return err.to_compile_error().into(),
        };
        let accessor: proc_macro2::TokenStream = if named {
            let ident = field.ident.as_ref().expect("named field");
            quote!(#ident)
        } else {
            let index = syn::Index::from(i);
            quote!(#index)
        };
        let ty = &field.ty;

        let clone_expr = if fattrs.skip {
            quote!(self.#accessor.clone())
        } else {
            quote!(*self.#accessor.reflect_clone().reflect_into_any().downcast::<#ty>().expect("field clone returns the field type"))
        };
        let apply_stmt = if fattrs.skip {
            quote!(self.#accessor = source.#accessor.clone();)
        } else {
            quote!(self.#accessor.reflect_apply(&source.#accessor)?;)
        };

        if named {
            clone_parts.push(quote!(#accessor: #clone_expr));
        } else {
            clone_parts.push(clone_expr);
        }
        apply_stmts.push(apply_stmt);

        if !fattrs.skip {
            let external = fattrs.rename.unwrap_or_else(|| {
                field
                    .ident
                    .as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| i.to_string())
            });
            table_accessors.push(accessor);
            table_names.push(external);
            table_tys.push(ty.clone());
        }
    }

    // named: `Self { f: expr, .. }`; tuple: `Self(expr, ..)` (clone_parts are
    // bare exprs for tuple structs, `f: expr` for named)
    let clone_body = if named {
        quote! { ::std::boxed::Box::new(Self { #(#clone_parts),* }) }
    } else {
        quote! { ::std::boxed::Box::new(Self( #(#clone_parts),* )) }
    };
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
            #(#apply_stmts)*
            ::core::result::Result::Ok(())
        }
    };

    let indices: Vec<usize> = (0..table_accessors.len()).collect();
    let shape_impl = quote! {
        impl #crate_path::ReflectStruct for #name {
            fn reflect_field_infos(&self) -> &'static [#crate_path::FieldInfo] {
                // OnceLock because type_info() is not const; the impl is
                // non-generic (enforced above), so a plain static is per-type.
                static FIELDS: ::std::sync::OnceLock<::std::vec::Vec<#crate_path::FieldInfo>> =
                    ::std::sync::OnceLock::new();
                FIELDS
                    .get_or_init(|| {
                        ::std::vec![
                            #(
                                #crate_path::FieldInfo {
                                    name: #table_names,
                                    type_info: #crate_path::type_info::<#table_tys>(),
                                },
                            )*
                        ]
                    })
                    .as_slice()
            }

            fn reflect_field_at(&self, index: usize) -> ::core::option::Option<&dyn #crate_path::Reflect> {
                match index {
                    #(#indices => ::core::option::Option::Some(&self.#table_accessors as &dyn #crate_path::Reflect),)*
                    _ => ::core::option::Option::None,
                }
            }

            fn reflect_field_at_mut(&mut self, index: usize) -> ::core::option::Option<&mut dyn #crate_path::Reflect> {
                match index {
                    #(#indices => ::core::option::Option::Some(&mut self.#table_accessors as &mut dyn #crate_path::Reflect),)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    };

    let mut out = base_impl(
        &input,
        &attrs,
        syn::Ident::new("Struct", proc_macro2::Span::call_site()),
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

#[proc_macro_derive(ReflectEnum, attributes(reflect))]
pub fn derive_reflect_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
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
