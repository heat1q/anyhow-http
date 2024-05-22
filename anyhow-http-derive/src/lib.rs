use proc_macro::{self, TokenStream};
use syn::Error;

mod http_error;

#[proc_macro_derive(HttpError, attributes(http_error, from))]
pub fn derive_from_rejection(input: TokenStream) -> TokenStream {
    syn::parse(input)
        .and_then(http_error::expand_http_error)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
