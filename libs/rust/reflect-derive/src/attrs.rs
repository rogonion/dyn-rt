//! `#[reflect(...)]` attribute parsing shared by all three derives.

use syn::punctuated::Punctuated;
use syn::{Attribute, Lit, Meta, parse_quote};

pub struct TypeAttrs {
    pub crate_path: syn::Path,
    pub name: Option<syn::LitStr>,
    pub new: bool,
}

pub fn lit_str(nv: &syn::MetaNameValue, what: &str) -> syn::Result<syn::LitStr> {
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

pub fn parse_type_attrs(attrs: &[Attribute]) -> syn::Result<TypeAttrs> {
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

pub struct FieldAttrs {
    pub rename: Option<String>,
    pub skip: bool,
}

pub fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
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
