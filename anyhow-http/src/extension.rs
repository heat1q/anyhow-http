use http::StatusCode;
use std::{borrow::Cow, result::Result as StdResult};

use crate::HttpError;

/// Extension trait to map the error variant of a [`Result`] to a [`HttpError`].
pub trait ResultExt {
    type Item;

    /// Maps a `Result<T, E>` to `Result<T, HttpError<R>>` by creating a [`HttpError`] with the
    /// specified status code wrapping the error contained [`Err`].
    ///
    /// # Example
    ///
    /// ```
    /// # use http::StatusCode;
    /// # use anyhow_http::{http_error, HttpError, ResultExt};
    ///
    /// let err = "nan".parse::<i32>()
    ///     .map_status(StatusCode::BAD_REQUEST)
    ///     .unwrap_err();
    /// assert_eq!(HttpError::from(err).status_code(), StatusCode::BAD_REQUEST);
    /// ```
    fn map_status(self, status_code: StatusCode) -> anyhow::Result<Self::Item>;

    /// Maps a `Result<T, E>` to `Result<T, HttpError<R>>` by creating a [`HttpError`] with the
    /// specified status code and reason wrapping the error contained [`Err`].
    ///
    /// # Example
    ///
    /// ```
    /// # use http::StatusCode;
    /// # use anyhow_http::{http_error, HttpError, ResultExt};
    ///
    /// let err = "nan".parse::<i32>()
    ///     .map_http_error(StatusCode::BAD_REQUEST, "invalid number")
    ///     .unwrap_err();
    /// let http_err = HttpError::from(err);
    /// assert_eq!(http_err.status_code(), StatusCode::BAD_REQUEST);
    /// assert_eq!(http_err.reason().unwrap(), "invalid number");
    /// ```
    fn map_http_error<S>(self, status_code: StatusCode, reason: S) -> anyhow::Result<Self::Item>
    where
        S: Into<Cow<'static, str>>;
}

impl<E, T> ResultExt for StdResult<T, E>
where
    E: Into<anyhow::Error> + Send + Sync + 'static,
{
    type Item = T;

    fn map_status(self, status_code: StatusCode) -> anyhow::Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(HttpError::from_err(e).with_status_code(status_code).into()),
        }
    }

    fn map_http_error<S>(self, status_code: StatusCode, reason: S) -> anyhow::Result<Self::Item>
    where
        S: Into<Cow<'static, str>>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(HttpError::from_err(e)
                .with_status_code(status_code)
                .with_reason(reason.into())
                .into()),
        }
    }
}

/// Extension trait to transform an [`Option`] to a [`HttpError`].
pub trait OptionExt {
    type Item;

    /// Transforms the `Option<T>` into a `Result<T, HttpError<R>>`, mapping `Some(v)` to
    /// `Ok(v)` and `None` to `Err(HttpError)` with status code.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::StatusCode;
    /// # use anyhow_http::{http_error, HttpError, OptionExt};
    ///
    /// let err = None::<()>.ok_or_status(StatusCode::BAD_REQUEST).unwrap_err();
    /// assert_eq!(HttpError::from(err).status_code(), StatusCode::BAD_REQUEST);
    /// ```
    fn ok_or_status(self, status_code: StatusCode) -> anyhow::Result<Self::Item>;
}

impl<T> OptionExt for std::option::Option<T> {
    type Item = T;

    fn ok_or_status(self, status_code: StatusCode) -> anyhow::Result<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(HttpError::from_status_code(status_code).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn http_err_ext_result_map_status() {
        let result: StdResult<(), _> = Err(anyhow!("error"));
        let http_result = result.map_status(StatusCode::BAD_REQUEST);

        let Err(e) = http_result else { unreachable!() };
        let e: HttpError = e.into();
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.source.unwrap().to_string(), "error".to_owned());
    }

    #[test]
    fn http_err_ext_result_map_http_error() {
        let s = "nan".parse::<i32>().map_status(StatusCode::BAD_REQUEST);
        let Err(e) = s else { unreachable!() };
        let e: HttpError = e.into();
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);

        let result = Err(anyhow!("error"));
        let http_result: anyhow::Result<()> =
            result.map_http_error(StatusCode::BAD_REQUEST, "invalid request");

        let Err(e) = http_result else { unreachable!() };
        let e: HttpError = e.into();
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.source.unwrap().to_string(), "error".to_owned());
        assert_eq!(e.reason, Some("invalid request".into()));
    }

    #[test]
    fn http_err_ext_option() {
        let opt: Option<()> = None;
        let http_result = opt.ok_or_status(StatusCode::BAD_REQUEST);

        let Err(e) = http_result else { unreachable!() };
        let e: HttpError = e.into();
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_none());
    }
}
