use http::StatusCode;
use proc_macro2::{self, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parenthesized, parse::ParseBuffer, spanned::Spanned, Field, Fields, Ident, Item, ItemEnum,
    ItemStruct, LitInt, LitStr, Variant,
};

macro_rules! spanned_err {
    ($item:ident, $err:literal) => {
        syn::Error::new_spanned($item, concat!("`#[derive(HttpError)]`: ", $err))
    };
}

pub(crate) fn expand_http_error(item: Item) -> syn::Result<TokenStream> {
    match item {
        Item::Struct(item) => expand_struct(item),
        Item::Enum(item) => expand_enum(item),
        item => Err(spanned_err!(item, "unsupported item")),
    }
}

fn expand_struct(item: ItemStruct) -> syn::Result<TokenStream> {
    Err(spanned_err!(item, "structs are currently unsupported"))
}

fn expand_enum(item: ItemEnum) -> syn::Result<TokenStream> {
    let variant_args = item
        .variants
        .iter()
        .map(|variant| {
            let args = Arg::parse_from_variant(variant)?;
            Ok((variant, args))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let impl_display_block = impl_display(&item.ident, &variant_args)?;
    let impl_from_http_error_block = impl_from_http_error(&item.ident, &variant_args)?;
    let impl_from_anyhow_error_block = impl_from_anyhow_error(&item.ident);
    let impl_from_source_block = impl_from_source(&item.ident, &variant_args)?;

    let output = quote! {
        #impl_display_block
        #impl_from_http_error_block
        #impl_from_anyhow_error_block
        #impl_from_source_block
    };

    Ok(output)
}

fn impl_display(ty: &Ident, variant_args: &[(&Variant, Arg)]) -> syn::Result<TokenStream> {
    let variants = variant_args
        .iter()
        .map(
            |(
                variant,
                arg,
            )| {
                let ident = &variant.ident;
                let ident = quote!{
                    ::core::stringify!(#ty::#ident)
                };
                let source_field = SourceField::parse_from_variant(variant)?;
                let span = variant.span();
                let lhs = quote_match_variant_lhs(ty, variant);
                let rhs = match (arg, &source_field) {
                    (Arg::Explicit { status_code, .. }, Some(SourceField { ident: sident, .. })) => {
                        quote_spanned! {span=>::core::write!(f, "http error {}: {}: {}", #status_code, #ident, #sident)}
                    },
                    (Arg::Explicit { status_code, .. }, _) => {
                        quote_spanned! {span=>::core::write!(f, "http error {}: {}", #status_code, #ident)}
                    },
                    (Arg::Transparent, Some(SourceField { ident: sident, .. })) => {
                        quote_spanned! {span=>#sident.fmt(f)}
                    },
                    (_,_) => {
                        unreachable!()
                    },
                };
                Ok(quote_spanned! {span=>#lhs => #rhs,})
            },
        )
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl ::std::fmt::Display for #ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#variants)*
                }
            }
        }
    })
}

fn quote_match_variant_lhs(ty: &Ident, variant: &Variant) -> TokenStream {
    let ident = &variant.ident;
    let span = variant.span();
    match &variant.fields {
        syn::Fields::Named(f) => {
            let f: Vec<_> = f
                .named
                .iter()
                .filter_map(|f| {
                    let lhs = f.ident.as_ref()?;
                    let rhs = format_ident!("__f_{}", lhs);
                    Some(quote! {#lhs: #rhs})
                })
                .collect();
            quote_spanned! {span=>#ty::#ident{#(#f,)*}}
        }
        syn::Fields::Unnamed(f) => {
            let f: Vec<_> = f
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| format_ident!("__f_{i}"))
                .collect();
            quote_spanned! {span=>#ty::#ident(#(#f,)*)}
        }
        syn::Fields::Unit => quote_spanned! {span=>#ty::#ident},
    }
}

fn impl_from_http_error(ty: &Ident, variant_args: &[(&Variant, Arg)]) -> syn::Result<TokenStream> {
    let variants = variant_args
        .iter()
        .map(|(variant, arg)| {
            let source_field = SourceField::parse_from_variant(variant)?;
            let lhs = quote_match_variant_lhs(ty, variant);
            let span = variant.span();
            let rhs = match (arg, &source_field) {
                (
                    Arg::Explicit {
                        status_code,
                        reason: Some(reason),
                        ..
                    },
                    Some(SourceField { ident: sident, .. }),
                ) => {
                    quote_spanned! {span=>
                        ::anyhow_http::HttpError::from_status_code(#status_code.try_into().unwrap())
                            .with_reason(::std::format!(#reason))
                            .with_source_err(#sident)
                    }
                }
                (
                    Arg::Explicit {
                        status_code,
                        reason: Some(reason),
                        ..
                    },
                    None,
                ) => {
                    quote_spanned! {span=>
                        ::anyhow_http::HttpError::from_status_code(#status_code.try_into().unwrap())
                            .with_reason(::std::format!(#reason))
                    }
                }
                (Arg::Explicit { status_code, .. }, Some(SourceField { ident: sident, .. })) => {
                    quote_spanned! {span=>
                        ::anyhow_http::HttpError::from_status_code(#status_code.try_into().unwrap())
                            .with_source_err(#sident)
                    }
                }
                (Arg::Explicit { status_code, .. }, _) => {
                    quote_spanned! {span=>
                        ::anyhow_http::HttpError::from_status_code(#status_code.try_into().unwrap())
                    }
                }
                (Arg::Transparent, Some(SourceField { ident: sident, .. })) => {
                    quote_spanned! {span=>
                        ::anyhow_http::HttpError::from_err(#sident)
                    }
                }
                (_, _) => {
                    unreachable!()
                }
            };
            Ok(quote_spanned! {span=>#lhs => #rhs,})
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #[allow(fallible_impl_from, clippy::useless_format)]
        impl ::std::convert::From<#ty> for ::anyhow_http::HttpError {
            fn from(e: #ty) -> Self {
                match e {
                    #(#variants)*
                }
            }
        }
    })
}

fn impl_from_anyhow_error(ty: &Ident) -> TokenStream {
    quote! {
        impl ::std::convert::From<#ty> for ::anyhow::Error {
            fn from(e: #ty) -> Self {
                ::anyhow_http::HttpError::from(e).into()
            }
        }
    }
}

fn impl_from_source(ty: &Ident, variant_args: &[(&Variant, Arg)]) -> syn::Result<TokenStream> {
    let mut from_impls = quote! {};
    for (variant, _) in variant_args {
        let Some(source_field) = SourceField::parse_from_variant(variant)? else {
            continue;
        };
        let sty = source_field.field.ty;
        let ident = &variant.ident;

        let from_source = quote! {
            impl ::std::convert::From<#sty> for #ty {
                fn from(s: #sty) -> Self {
                    Self::#ident(s)
                }
            }
        };

        from_impls = quote! {
            #from_impls
            #from_source
        };
    }

    Ok(from_impls)
}

#[derive(Debug)]
enum Arg {
    Explicit {
        status_code: LitInt,
        reason: Option<String>,
    },
    Transparent,
}

impl Arg {
    fn parse_from_variant(variant: &Variant) -> syn::Result<Self> {
        let mut status_code = None;
        let mut reason = None;
        let mut transparent = false;
        let attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("http_error"))
            .ok_or_else(|| spanned_err!(variant, "missing `http_error` attribute"))?;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("status") {
                let content;
                parenthesized!(content in meta.input);
                status_code = Some(Self::parse_and_validate_status_code(variant, &content)?);
                return Ok(());
            }

            if meta.path.is_ident("reason") {
                let content;
                parenthesized!(content in meta.input);
                reason = Some(Self::parse_reason(&content)?);
                return Ok(());
            }

            if meta.path.is_ident("transparent") {
                transparent = true;
                return Ok(());
            }

            Err(meta.error("unrecognized argument to `#[http_error(..)]`"))
        })?;

        if transparent {
            if status_code.is_some() || reason.is_some() {
                return Err(spanned_err!(
                    variant,
                    "`#[http_error(transparent)]` may not use `status` or `reason`"
                ));
            }

            return Ok(Self::Transparent);
        }

        let Some(status_code) = status_code else {
            return Err(spanned_err!(
                variant,
                "missing `#[http_error(status(..))]` attribute"
            ));
        };

        Ok(Self::Explicit {
            status_code,
            reason,
        })
    }

    fn parse_and_validate_status_code(variant: &Variant, buf: &ParseBuffer) -> syn::Result<LitInt> {
        let lit: LitInt = buf.parse()?;
        let status_code: u16 = lit.base10_parse()?;
        StatusCode::try_from(status_code)
            .map_err(|_| spanned_err!(variant, "invalid status code"))?;
        Ok(lit)
    }

    fn parse_reason(buf: &ParseBuffer) -> syn::Result<String> {
        let reason: LitStr = buf.parse()?;
        let mut format = String::new();
        for c in reason.value().chars() {
            format.push(c);
            if c == '{' {
                format.push_str("__f_");
            }
        }
        Ok(format)
    }
}

#[derive(Debug)]
struct SourceField {
    ident: Ident,
    field: Field,
}

impl SourceField {
    fn parse_from_variant(variant: &Variant) -> syn::Result<Option<Self>> {
        let source = match &variant.fields {
            Fields::Named(_) => None,
            Fields::Unnamed(f) => {
                let Some(field) = f
                    .unnamed
                    .iter()
                    .find(|f| f.attrs.iter().any(|a| a.path().is_ident("from")))
                else {
                    return Ok(None);
                };

                if f.unnamed.len() > 1 {
                    return Err(spanned_err!(
                        variant,
                        "`#[from]` is only supported on single unnamed fields"
                    ));
                }

                Some(Self {
                    ident: format_ident!("__f_0"),
                    field: field.clone(),
                })
            }
            Fields::Unit => None,
        };

        Ok(source)
    }
}
