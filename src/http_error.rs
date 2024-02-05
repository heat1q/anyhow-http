use core::fmt;
use fmt::Debug;
use std::collections::HashMap;
use std::marker::PhantomData;

use http::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::response::{HttpErrorResponse, IntoHttpErrorResponse};

/// [`HttpError`] is an error that can be represented as a HTTP response. [`HttpError`] is generic over
/// its response format, allowing consumers to implement their custom error response. See [`IntoHttpErrorResponse`].
#[derive(Debug)]
pub struct HttpError<R> {
    pub status_code: StatusCode,
    pub reason: Option<String>,
    pub source: Option<anyhow::Error>,
    pub data: HashMap<String, serde_json::Value>,
    _formatter: PhantomData<R>,
}

impl<R> fmt::Display for HttpError<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.reason, &self.source) {
            (None, None) => write!(f, "http error {}", self.status_code),
            (None, Some(s)) => write!(f, "http error {}, source: {s}", self.status_code),
            (Some(r), None) => write!(f, "http error {}: {r}", self.status_code),
            (Some(r), Some(s)) => write!(f, "http error {}: {r}, source: {s}", self.status_code),
        }
    }
}

impl<R> Default for HttpError<R> {
    fn default() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            reason: None,
            source: None,
            data: HashMap::default(),
            #[allow(clippy::default_constructed_unit_structs)]
            _formatter: PhantomData::default(),
        }
    }
}

#[derive(Debug)]
struct DynErrorWrapper(Box<dyn std::error::Error + Send + Sync + 'static>);

impl fmt::Display for DynErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DynErrorWrapper {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

impl<R> HttpError<R>
where
    R: fmt::Debug + Sync + Send + 'static,
{
    /// Creates a [`HttpError`] from a status code.
    pub fn from_status_code(status_code: StatusCode) -> Self {
        Self {
            status_code,
            ..Default::default()
        }
    }

    /// Creates a [`HttpError`] from another [`HttpError`]. This is mostly used to convert between
    /// [`HttpError`]s with different response formats.
    pub fn from_http_err<S>(http_err: HttpError<S>) -> Self {
        Self {
            status_code: http_err.status_code,
            reason: http_err.reason,
            source: http_err.source,
            ..Self::default()
        }
    }

    /// Creates a [`HttpError`] from a generic error. It attempts to downcast to an underlying
    /// [`HttpError`].
    pub fn from_err<E>(err: E) -> Self
    where
        E: Into<anyhow::Error>,
    {
        let err = err.into();
        // TODO: bridge from HttpError to anyhow::Error first
        match err.downcast::<HttpError<R>>() {
            Ok(http_error) => http_error,
            Err(err) => Self {
                source: Some(err),
                ..Self::default()
            },
        }
    }

    /// Sets the status code.
    pub fn with_status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    /// Sets the error reason.
    pub fn with_reason<S: ToString>(mut self, reason: S) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    /// Set the source error from a generic error trait object.
    pub fn with_dyn_source_err(
        mut self,
        err: Box<dyn std::error::Error + Send + Sync + 'static>,
    ) -> Self {
        self.source = Some(DynErrorWrapper(err).into());
        self
    }

    /// Set the source error from a generic error.
    pub fn with_source_err<E>(mut self, err: E) -> Self
    where
        E: Into<anyhow::Error>,
    {
        self.source = Some(err.into());
        self
    }

    /// Adds a key-pair value to the inner data.
    pub fn add<V>(&mut self, key: impl Into<String>, value: V) -> serde_json::Result<()>
    where
        V: Serialize + Send + Sync,
    {
        self.data.insert(key.into(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Retrieves a key-pair value from the inner data.
    pub fn get<V>(&self, key: impl AsRef<str>) -> Option<V>
    where
        V: DeserializeOwned + Send + Sync,
    {
        self.data
            .get(key.as_ref())
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

impl<R> HttpError<R>
where
    R: IntoHttpErrorResponse<Fmt = R> + fmt::Debug + Send + Sync + 'static,
{
    /// Creates an error response. See [`IntoHttpErrorResponse`].
    pub fn into_http_error_response(self) -> HttpErrorResponse<R> {
        R::into_http_error_response(self)
    }
}

impl<E, R> From<E> for HttpError<R>
where
    E: Into<anyhow::Error>,
    R: fmt::Debug + Sync + Send + 'static,
{
    fn from(err: E) -> Self {
        HttpError::from_err(err)
    }
}

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
impl<R> axum::response::IntoResponse for HttpError<R>
where
    R: IntoHttpErrorResponse<Fmt = R> + fmt::Debug + Send + Sync + 'static,
{
    fn into_response(self) -> axum::response::Response {
        R::into_http_error_response(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn http_error_display() {
        let e: HttpError<()> = HttpError::default();
        assert_eq!(e.to_string(), "http error 500 Internal Server Error");

        let e: HttpError<()> = HttpError::default().with_reason("reason");
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error: reason"
        );

        let e: HttpError<()> = HttpError::default().with_source_err(anyhow!("error"));
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error, source: error"
        );

        let e: HttpError<()> = HttpError::default()
            .with_reason("reason")
            .with_source_err(anyhow!("error"));
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error: reason, source: error"
        );
    }

    #[test]
    fn http_error_default() {
        let e: HttpError<()> = HttpError::default();
        assert_eq!(e.status_code, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn http_error_from_status_code() {
        let e: HttpError<()> = HttpError::from_status_code(StatusCode::BAD_REQUEST);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_from_http_error() {
        let _e: HttpError<()> = HttpError::from_http_err::<i32>(HttpError::default());
    }

    #[test]
    fn http_error_from_err() {
        let e: HttpError<()> = HttpError::from_err(anyhow!("error"));
        assert_eq!(e.status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.source.unwrap().to_string(), "error");

        let e: HttpError<()> = HttpError::from_err(fmt::Error);
        assert_eq!(e.status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.source.unwrap().to_string(), fmt::Error.to_string());
    }

    #[test]
    fn http_error_from_custom_impl_try() {
        struct GenericError;
        impl<R> From<GenericError> for HttpError<R>
        where
            R: fmt::Debug + Sync + Send + 'static,
        {
            fn from(_: GenericError) -> Self {
                Self::default().with_status_code(StatusCode::BAD_REQUEST)
            }
        }

        let e: std::result::Result<(), HttpError<()>> = (|| {
            Err(GenericError)?;
            unreachable!()
        })();
        assert_eq!(e.unwrap_err().status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_with_status_code() {
        let e: HttpError<()> = HttpError::default().with_status_code(StatusCode::BAD_REQUEST);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_with_reason() {
        let e: HttpError<()> = HttpError::default().with_reason("reason");
        assert_eq!(e.reason, Some("reason".to_string()));
    }

    #[test]
    fn http_error_with_dyn_source_error() {
        let dyn_err = Box::new(fmt::Error) as Box<dyn std::error::Error + Send + Sync + 'static>;
        let e: HttpError<()> = HttpError::default().with_dyn_source_err(dyn_err);
        assert_eq!(e.source.unwrap().to_string(), fmt::Error.to_string());
    }

    #[test]
    fn http_error_with_source_error() {
        let e: HttpError<()> = HttpError::default().with_source_err(fmt::Error);
        assert_eq!(e.source.unwrap().to_string(), fmt::Error.to_string());
    }

    #[test]
    fn http_error_data() {
        let mut e: HttpError<()> = HttpError::default();
        e.add("key", 1234).unwrap();
        assert_eq!(e.get::<i32>("key"), Some(1234));
        assert_eq!(e.get::<String>("key"), None);
    }
}
