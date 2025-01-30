//! Creating responses from [`HttpError`].
use bytes::Bytes;
use std::marker::PhantomData;

use crate::http_error::HttpError;

/// A result that wraps [`HttpError`] with response formatter [`FormatResponse`].
pub type HttpResult<T, F> = core::result::Result<T, HttpErrorResponse<F>>;

/// Type representing an error response.
#[derive(Debug)]
pub struct HttpErrorResponse<F: FormatResponse> {
    pub http_error: HttpError,
    _formatter: PhantomData<F>,
}

impl<E, F> From<E> for HttpErrorResponse<F>
where
    F: FormatResponse,
    E: Into<anyhow::Error>,
{
    fn from(e: E) -> Self {
        Self {
            http_error: HttpError::from_err(e),
            _formatter: PhantomData,
        }
    }
}

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
impl<F: FormatResponse> axum::response::IntoResponse for HttpErrorResponse<F> {
    fn into_response(self) -> axum::response::Response {
        let mut headers = self.http_error.headers.clone().unwrap_or_default();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_str(F::content_type().as_ref()).unwrap(),
        );
        let mut resp = (
            self.http_error.status_code,
            headers,
            F::format_response(&self.http_error),
        )
            .into_response();
        resp.extensions_mut()
            .insert(std::sync::Arc::new(self.http_error));
        resp
    }
}

/// Trait for formatting error responses.
pub trait FormatResponse {
    fn format_response(http_error: &HttpError) -> Bytes;
    fn content_type() -> mime::Mime;
}

/// A [`HttpErrorResponse`] with configured [`Json`] formatter.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub type HttpJsonErrorResponse = HttpErrorResponse<Json>;

/// A [`HttpResult`] with configured [`Json`] formatter.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub type HttpJsonResult<T> = core::result::Result<T, HttpJsonErrorResponse>;

/// A general purpose error response that formats a [`HttpError`] as Json.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
#[derive(Debug)]
pub struct Json;

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl FormatResponse for Json {
    fn format_response(http_error: &HttpError) -> Bytes {
        use bytes::BufMut;
        let error_reason = http_error
            .reason()
            .as_deref()
            .or_else(|| http_error.status_code().canonical_reason())
            .map(String::from);

        let mut resp = serde_json::json!({
            "error": {
                "message": error_reason,
            },
        });
        if let Some(data) = &http_error.data {
            for (k, v) in data {
                resp["error"][k] = v.clone();
            }
        }

        let mut buf = bytes::BytesMut::with_capacity(128).writer();
        if let Err(err) = serde_json::to_writer(&mut buf, &resp) {
            return err.to_string().into();
        }

        buf.into_inner().freeze()
    }

    fn content_type() -> mime::Mime {
        mime::APPLICATION_JSON
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_error;
    use http::StatusCode;

    #[test]
    #[cfg(feature = "json")]
    fn http_error_response_json() {
        let resp: HttpErrorResponse<Json> = http_error!(BAD_REQUEST).into();
        assert_eq!(resp.http_error.status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    #[cfg(all(feature = "axum", feature = "json"))]
    fn http_error_resonse_axum_into_response() {
        use axum::response::IntoResponse;
        let resp: HttpErrorResponse<Json> = HttpError::from_status_code(StatusCode::BAD_REQUEST)
            .with_header("x-custom-header", 42.into())
            .into();
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get("content-type"),
            Some(&"application/json".parse().unwrap())
        );
        assert_eq!(resp.headers().get("x-custom-header"), Some(&42.into()));
    }

    #[test]
    #[cfg(feature = "json")]
    fn http_error_json_response() {
        let e: HttpError = http_error!(BAD_REQUEST, "invalid param",);
        let e = e.with_key_value("ctx", "some context");
        let e = e.with_key_value("code", 1234);
        let body = Json::format_response(&e);
        let content_type = Json::content_type();
        assert_eq!(
            body,
            Bytes::from_static(
                b"{\"error\":{\"code\":1234,\"ctx\":\"some context\",\"message\":\"invalid param\"}}"
            )
        );
        assert_eq!(content_type, mime::APPLICATION_JSON);
    }

    #[test]
    #[cfg(feature = "json")]
    fn http_error_response_from_anyhow_downcast() {
        let res: HttpResult<(), Json> = (|| {
            let e: anyhow::Error = http_error!(BAD_REQUEST).into();
            Err(e)?;
            unreachable!()
        })();
        let e = res.unwrap_err().http_error;
        assert_eq!(e.status_code(), StatusCode::BAD_REQUEST)
    }
}
