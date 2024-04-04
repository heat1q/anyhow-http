//! Creating responses from [`HttpError`].
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use serde::Serialize;
use serde_json::json;
use std::fmt;

use crate::http_error::HttpError;

/// A result that wraps [`HttpError`].
pub type Result<T> = core::result::Result<T, HttpError<Json>>;

/// Type representing an error response.
#[derive(Debug)]
#[allow(dead_code)]
pub struct HttpErrorResponse<R: fmt::Debug> {
    pub(crate) http_error: HttpError<R>,
    pub(crate) body: Bytes,
    pub(crate) content_type: mime::Mime,
}

impl<R: fmt::Debug> HttpErrorResponse<R> {
    /// Constructs a plain text error response for the given [`HttpError`].
    pub fn plain(http_error: HttpError<R>, body: impl Into<Bytes>) -> Self {
        HttpErrorResponse {
            http_error,
            body: body.into(),
            content_type: mime::TEXT_PLAIN,
        }
    }

    /// Constructs a Json error response for the given [`HttpError`].
    pub fn json(http_error: HttpError<R>, body: impl Serialize) -> Self {
        let mut buf = BytesMut::with_capacity(128).writer();
        if let Err(err) = serde_json::to_writer(&mut buf, &body) {
            return Self::plain(http_error, err.to_string());
        }

        HttpErrorResponse {
            http_error,
            body: buf.into_inner().freeze(),
            content_type: mime::APPLICATION_JSON,
        }
    }

    /// Constructs a Html error response for the given [`HttpError`].
    pub fn html(http_error: HttpError<R>, body: impl Into<Bytes>) -> Self {
        HttpErrorResponse {
            http_error,
            body: body.into(),
            content_type: mime::TEXT_HTML,
        }
    }
}

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
impl<R> axum::response::IntoResponse for HttpErrorResponse<R>
where
    R: fmt::Debug + Send + Sync + 'static,
{
    fn into_response(self) -> axum::response::Response {
        let mut resp = (
            self.http_error.status_code,
            [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_str(self.content_type.as_ref()).unwrap(),
            )],
            self.body,
        )
            .into_response();
        resp.extensions_mut()
            .insert(std::sync::Arc::new(self.http_error));
        resp
    }
}

/// Trait for generating error responses.
///
/// Types that implement `IntoHttpErrorResponse` are used as generic argument to [`HttpError`].
pub trait IntoHttpErrorResponse {
    /// The error response format.
    type Fmt: fmt::Debug + Send + Sync;
    /// Creates an error response.
    fn into_http_error_response(http_error: HttpError<Self::Fmt>) -> HttpErrorResponse<Self::Fmt>;
}

/// A general purpose error response that formats a [`HttpError`] as Json.
#[derive(Debug)]
pub struct Json;

impl IntoHttpErrorResponse for Json {
    type Fmt = Json;

    fn into_http_error_response(http_error: HttpError<Self::Fmt>) -> HttpErrorResponse<Self::Fmt> {
        let error_reason = http_error
            .reason()
            .as_deref()
            .or_else(|| http_error.status_code().canonical_reason())
            .map(String::from);

        let mut resp = json!({
            "error": error_reason,
        });
        if let Some(data) = &http_error.data {
            for (k, v) in data {
                resp[k] = v.clone();
            }
        }

        HttpErrorResponse::json(http_error, resp)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::http_error;

    use super::*;

    #[test]
    fn http_error_response_json() {
        let resp: HttpErrorResponse<()> =
            HttpErrorResponse::json(http_error!(BAD_REQUEST), serde_json::Value::Array(vec![]));
        assert_eq!(resp.http_error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(resp.body, Bytes::from_static(b"[]"));
        assert_eq!(resp.content_type, mime::APPLICATION_JSON);
    }

    #[test]
    #[cfg(feature = "axum")]
    fn http_error_resonse_axum_into_response() {
        use axum::response::IntoResponse;
        let resp: HttpErrorResponse<()> =
            HttpErrorResponse::json(http_error!(BAD_REQUEST), serde_json::Value::Array(vec![]));
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_json_response() {
        let mut e: HttpError<Json> = http_error!(BAD_REQUEST, "invalid param",);
        e.add("ctx", "some context").unwrap();
        e.add("code", 1234).unwrap();
        let resp = e.into_http_error_response();
        assert_eq!(resp.http_error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.body,
            Bytes::from_static(
                b"{\"code\":1234,\"ctx\":\"some context\",\"error\":\"invalid param\"}"
            )
        );
        assert_eq!(resp.content_type, mime::APPLICATION_JSON);
    }
}
