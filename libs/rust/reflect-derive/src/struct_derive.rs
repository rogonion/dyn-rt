//! `#[derive(ReflectStruct)]` expansion: base `Reflect` impl (routing
//! `Struct`) + the `ReflectStruct` shape impl.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{parse_field_attrs, parse_type_attrs};
use crate::base::{base_impl, new_impl};

pub fn expand(input: DeriveInput) -> TokenStream {
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
