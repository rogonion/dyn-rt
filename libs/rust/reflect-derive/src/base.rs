//! Base `Reflect` impl generation shared by all three derives, plus the
//! `#[derive(Reflect)]` (base-only, `Unknown`) expansion.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, parse_quote};

use crate::attrs::{TypeAttrs, parse_type_attrs};

/// Generate the base `Reflect` impl. `route` is `Unknown`, `Struct` or `Enum`;
/// `clone_body` / `apply_body` are the per-shape bodies; `require_clone` adds a
/// `Self: Clone` where-predicate (base derive only).
pub fn base_impl(
    input: &DeriveInput,
    attrs: &TypeAttrs,
    route: syn::Ident,
    clone_body: TokenStream2,
    apply_body: TokenStream2,
    require_clone: bool,
) -> TokenStream2 {
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

pub fn new_impl(input: &DeriveInput, attrs: &TypeAttrs) -> TokenStream2 {
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

/// `#[derive(Reflect)]` — base only, routes to `Unknown`, `Clone`-based.
pub fn expand_reflect(input: DeriveInput) -> TokenStream {
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
