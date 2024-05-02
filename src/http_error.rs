use anyhow::anyhow;
use core::fmt;
use fmt::Debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error as StdError;
use std::{borrow::Cow, collections::HashMap};

use http::StatusCode;

/// [`HttpError`] is an error that can be represented as a HTTP response. [`HttpError`] is generic over
/// its response format, allowing consumers to implement their custom error response. See [`IntoHttpErrorResponse`].
#[derive(Debug)]
pub struct HttpError {
    pub(crate) status_code: StatusCode,
    pub(crate) reason: Option<Cow<'static, str>>,
    pub(crate) source: Option<anyhow::Error>,
    pub(crate) data: Option<HashMap<String, serde_json::Value>>,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.reason, &self.source) {
            (None, None) => write!(f, "http error {}", self.status_code),
            (None, Some(s)) => write!(f, "http error {}, source: {s}", self.status_code),
            (Some(r), None) => write!(f, "http error {}: {r}", self.status_code),
            (Some(r), Some(s)) => write!(f, "http error {}: {r}, source: {s}", self.status_code),
        }
    }
}

impl StdError for HttpError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn StdError + 'static))
    }
}

#[allow(clippy::derivable_impls)]
impl Default for HttpError {
    fn default() -> Self {
        Self { ..Self::new() }
    }
}

impl PartialEq for HttpError {
    fn eq(&self, other: &Self) -> bool {
        self.status_code == other.status_code
            && self.reason == other.reason
            && self.data == other.data
    }
}

impl HttpError {
    /// Creates an empty [`HttpError`] with status 500.
    pub const fn new() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            reason: None,
            source: None,
            data: None,
        }
    }

    /// Creates a [`HttpError`] with status code and reason. This constructor can be evaluated at
    /// compile time.
    ///
    /// ```rust
    /// use anyhow_http::HttpError;
    ///
    /// const BAD_REQUEST: HttpError =
    ///     HttpError::from_static(http::StatusCode::BAD_REQUEST, "invalid request");
    /// ```
    pub const fn from_static(status_code: StatusCode, reason: &'static str) -> Self {
        Self {
            status_code,
            reason: Some(Cow::Borrowed(reason)),
            source: None,
            data: None,
        }
    }

    /// Creates a [`HttpError`] from a status code.
    pub const fn from_status_code(status_code: StatusCode) -> Self {
        let mut http_err = Self::new();
        http_err.status_code = status_code;
        http_err
    }

    /// Sets the status code.
    pub const fn with_status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    /// Sets the error reason.
    pub fn with_reason<S: Into<Cow<'static, str>>>(mut self, reason: S) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the source error from a generic error trait object.
    pub fn with_boxed_source_err(mut self, err: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        self.source = Some(anyhow!("{err}"));
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

    /// Append to to the inner data based on one or more key-value pairs.
    ///
    /// ```rust
    /// use anyhow_http::HttpError;
    ///
    /// let err: HttpError = HttpError::default()
    ///     .with_data([("key1", 1234), ("key2", 5678)])
    ///     .unwrap();
    /// ```
    pub fn with_data<I, K, V>(mut self, values: I) -> Option<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Serialize + Sync + Send + 'static,
    {
        let iter = values
            .into_iter()
            .map(|(k, v)| Some((k.into(), serde_json::to_value(v).ok()?)));

        self.data = self
            .data
            .get_or_insert_with(HashMap::new)
            .clone()
            .into_iter()
            .map(Option::Some)
            .chain(iter)
            .collect();

        Some(self)
    }

    /// Adds a key-pair value to the inner data.
    pub fn add<K, V>(&mut self, key: K, value: V) -> serde_json::Result<()>
    where
        K: Into<String>,
        V: Serialize + Sync + Send + 'static,
    {
        self.data
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Retrieves a key-pair value from the inner data.
    pub fn get<V>(&self, key: impl AsRef<str>) -> Option<V>
    where
        V: DeserializeOwned + Send + Sync,
    {
        self.data
            .as_ref()
            .and_then(|d| d.get(key.as_ref()))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Returns the status code.
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// Returns the error reason if any.
    pub fn reason(&self) -> Option<Cow<'static, str>> {
        self.reason.clone()
    }

    /// Returns the source error if any.
    pub fn source(&self) -> Option<&anyhow::Error> {
        self.source.as_ref()
    }

    /// Creates a [`HttpError`] from a generic error. It attempts to downcast to an underlying
    /// [`HttpError`].
    pub fn from_err<E>(err: E) -> Self
    where
        E: Into<anyhow::Error>,
    {
        let err = err.into();
        match err.downcast::<HttpError>() {
            Ok(http_error) => http_error,
            Err(err) => Self {
                source: Some(err),
                ..Self::default()
            },
        }
    }

    pub fn into_boxed(self) -> Box<dyn StdError + Send + Sync + 'static> {
        self.into()
    }
}

impl From<anyhow::Error> for HttpError {
    fn from(err: anyhow::Error) -> Self {
        HttpError::from_err(err)
    }
}

//#[cfg(feature = "axum")]
//#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
//impl<R> axum::response::IntoResponse for HttpError
//where
//    R: IntoHttpErrorResponse<Fmt = R> + fmt::Debug + Send + Sync + 'static,
//{
//    fn into_response(self) -> axum::response::Response {
//        R::into_http_error_response(self).into_response()
//    }
//}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn http_error_display() {
        let e: HttpError = HttpError::default();
        assert_eq!(e.to_string(), "http error 500 Internal Server Error");

        let e: HttpError = HttpError::default().with_reason("reason");
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error: reason"
        );

        let e: HttpError = HttpError::default().with_source_err(anyhow!("error"));
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error, source: error"
        );

        let e: HttpError = HttpError::default()
            .with_reason("reason")
            .with_source_err(anyhow!("error"));
        assert_eq!(
            e.to_string(),
            "http error 500 Internal Server Error: reason, source: error"
        );
    }

    #[test]
    fn http_error_new_const() {
        const ERR: HttpError = HttpError::new();
        assert_eq!(ERR.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn http_error_from_static() {
        const ERR: HttpError = HttpError::from_static(StatusCode::BAD_REQUEST, "invalid request");
        assert_eq!(ERR.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ERR.reason(), Some("invalid request".into()));
    }

    #[test]
    fn http_error_default() {
        let e: HttpError = HttpError::default();
        assert_eq!(e.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn http_error_from_status_code() {
        let e: HttpError = HttpError::from_status_code(StatusCode::BAD_REQUEST);
        assert_eq!(e.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_from_err() {
        let e: HttpError = HttpError::from_err(anyhow!("error"));
        assert_eq!(e.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.source().unwrap().to_string(), "error");

        let e: HttpError = HttpError::from_err(fmt::Error);
        assert_eq!(e.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.source().unwrap().to_string(), fmt::Error.to_string());
    }

    #[derive(Debug)]
    struct GenericError;
    impl std::fmt::Display for GenericError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "CustomError")
        }
    }

    impl From<GenericError> for HttpError {
        fn from(_: GenericError) -> Self {
            Self::default().with_status_code(StatusCode::BAD_REQUEST)
        }
    }

    impl From<GenericError> for anyhow::Error {
        fn from(_: GenericError) -> Self {
            HttpError::default()
                .with_status_code(StatusCode::BAD_REQUEST)
                .into()
        }
    }

    #[test]
    fn http_error_from_custom_impl_try() {
        let res: std::result::Result<(), HttpError> = (|| {
            Err(GenericError)?;
            unreachable!()
        })();
        let e = res.unwrap_err();
        assert_eq!(e.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_into_anyhow() {
        let res: anyhow::Result<()> = (|| {
            Err(GenericError)?;
            unreachable!()
        })();
        let e = res.unwrap_err();
        assert_eq!(
            HttpError::from_err(e).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn http_error_with_status_code() {
        let e: HttpError = HttpError::default().with_status_code(StatusCode::BAD_REQUEST);
        assert_eq!(e.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_with_reason() {
        let e: HttpError = HttpError::default().with_reason("reason");
        assert_eq!(e.reason(), Some("reason".into()));
    }

    #[test]
    fn http_error_with_dyn_source_error() {
        let dyn_err = Box::new(fmt::Error) as Box<dyn StdError + Send + Sync + 'static>;
        let e: HttpError = HttpError::default().with_boxed_source_err(dyn_err);
        assert_eq!(e.source().unwrap().to_string(), fmt::Error.to_string());
    }

    #[test]
    fn http_error_with_source_error() {
        let e: HttpError = HttpError::default().with_source_err(fmt::Error);
        assert_eq!(e.source().unwrap().to_string(), fmt::Error.to_string());
    }

    #[test]
    fn http_error_data() {
        let mut e: HttpError = HttpError::default();
        e.add("key", 1234).unwrap();
        assert_eq!(e.get::<i32>("key"), Some(1234));
        assert_eq!(e.get::<String>("key"), None);
    }

    #[test]
    fn http_error_with_data() {
        let e: HttpError = HttpError::default()
            .with_data([("key1", 1234), ("key2", 5678)])
            .unwrap();
        assert_eq!(e.get::<i32>("key1"), Some(1234));
        assert_eq!(e.get::<i32>("key2"), Some(5678));
    }

    #[test]
    fn http_error_anyhow_downcast() {
        let outer: anyhow::Error = HttpError::from_status_code(StatusCode::BAD_REQUEST).into();
        let e = HttpError::from(outer);
        assert_eq!(e.status_code(), StatusCode::BAD_REQUEST);
    }
}
