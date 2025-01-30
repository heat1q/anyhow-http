use proc_macro::{self, TokenStream};
use syn::Error;

mod http_error;

/// Derives a [`From`] implementation for dedicated errors that behave like
/// [`HttpError`]s
///
/// This derive macro is similar to [`thiserror`] just for [`HttpError`]s.
///
/// **Note: Currently only enums are supported.**
///
/// Dedicated [`HttpError`]-like types are a good fit in places where errors need to be statically
/// defined and reused. Furthermore, they help make the code more readable and maintainable.
///
/// Consider the following example where we would like to define a dedicated error for a specific
/// error case (request failed). We want the error to behave as a [`HttpError`] because it should
/// be passed along and eventually produce a specific Http response. In this case we want the
/// `RequestFailed` variant to emit a error response with status `502` and `reason` `request failed`.
/// [`FromHttpError`] derives a [`From`] implementation that allows the `CustomError` to be
/// converted into [`HttpError`] (and `anyhow::Error` consequently) according to the attributes we
/// define with `#[http_error(..)]`.
/// ```
/// # use anyhow::Result;
/// # use anyhow_http_derive::FromHttpError;
/// # use bytes::Bytes;
/// # use http::StatusCode;
/// # use std::future::Future;
/// #[derive(FromHttpError)]
/// enum CustomError {
///     #[http_error(status(502), reason("request failed"))]
///     RequestFailed,
/// }
///
/// async fn process_request(req: impl Future<Output = Result<Bytes>>) -> anyhow::Result<Bytes> {
///     let resp = req
///         .await
///         .map_err(|_| CustomError::RequestFailed)?;
///
///     Ok(resp)
/// }
/// ```
///
/// Supported arguments to the `#[http_error(..)]` attribute are `status`, `reason` and `data`.
/// `data` allows to set one or more key-value pairs to the [`HttpError`]'s data. Values may
/// be literals and any valid expressions.
/// ```
/// # use anyhow_http_derive::FromHttpError;
/// #[derive(FromHttpError)]
/// enum CustomError {
///     #[http_error(status(502), data(code = 1234, ctx = "more ctx"))]
///     RequestFailed,
/// }
/// ```
///
/// Similar to [`thiserror`] a `#[from]` attribute is provided to automatically generate a
/// [`From`] implementation for the specific variant. `#[from]` also sets the source of the
/// resulting [`HttpError`]. If only the source should be set without generating a [`From`]
/// implementation `#[source]` should be set.
/// ```
/// # use anyhow_http_derive::FromHttpError;
/// #[derive(FromHttpError)]
/// enum CustomError {
///     #[http_error(status(502), reason("request failed"))]
///     RequestFailed(#[from] anyhow::Error),
/// }
/// ```
///
/// Formatting on the `reason(..)` and `data(..)` attribute is supported on both named and unnamed
/// variants.
/// ```
/// # use anyhow_http_derive::FromHttpError;
/// #[derive(FromHttpError)]
/// enum CustomError {
///     #[http_error(status(502), reason("named: {ctx}"))]
///     Named { ctx: String },
///     #[http_error(status(502), reason("unnamed: {0}"))]
///     Unnamed(String),
/// }
/// ```
///
/// `transparent` allows to forward the source error as-is. It required either `#[source]` or
/// `#[from]`.
/// ```
/// # use anyhow_http_derive::FromHttpError;
/// #[derive(FromHttpError)]
/// enum CustomError {
///     #[http_error(transparent)]
///     Inner(#[source] anyhow::Error)
/// }
/// ```
///
/// [`From`]: std::convert::From
/// [`HttpError`]: https://docs.rs/anyhow-http/latest/anyhow_http/struct.HttpError.html
/// [`thiserror`]: https://docs.rs/thiserror/latest/thiserror/#derives
#[proc_macro_derive(FromHttpError, attributes(http_error, from, source, data))]
pub fn derive_from_http_error(input: TokenStream) -> TokenStream {
    syn::parse(input)
        .and_then(http_error::expand_http_error)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
