use http::StatusCode;
use std::{borrow::Cow, result::Result as StdResult};

use crate::HttpError;

/// Extension trait to map the error variant of a [`Result`] to a [`HttpError`].
pub trait ResultExt<R> {
    type Item;

    /// Maps a `Result<T, E>` to `Result<T, HttpError<R>>` by creating a [`HttpError`] with the
    /// specified status code wrapping the error contained [`Err`].
    ///
    /// # Example
    ///
    /// ```
    /// use http::StatusCode;
    /// use anyhow_http::{http_error, HttpError, ResultExt};
    ///
    /// let s: Result<i32, HttpError<()>> = "nan"
    ///     .parse::<i32>()
    ///     .map_status(StatusCode::BAD_REQUEST);
    /// assert_eq!(s, Err(http_error!(BAD_REQUEST)));
    /// ```
    fn map_status(self, status_code: StatusCode) -> StdResult<Self::Item, HttpError<R>>;

    /// Maps a `Result<T, E>` to `Result<T, HttpError<R>>` by creating a [`HttpError`] with the
    /// specified status code and reason wrapping the error contained [`Err`].
    ///
    /// # Example
    ///
    /// ```
    /// use http::StatusCode;
    /// use anyhow_http::{http_error, HttpError, ResultExt};
    ///
    /// let s: Result<i32, HttpError<()>> = "nan"
    ///     .parse::<i32>()
    ///     .map_http_error(StatusCode::BAD_REQUEST, "invalid number");
    /// assert_eq!(s, Err(http_error!(BAD_REQUEST, "invalid number")));
    /// ```
    fn map_http_error<S>(
        self,
        status_code: StatusCode,
        reason: S,
    ) -> StdResult<Self::Item, HttpError<R>>
    where
        S: Into<Cow<'static, str>>;
}

impl<E, R, T> ResultExt<R> for StdResult<T, E>
where
    E: Into<HttpError<R>> + Send + Sync + 'static,
    R: std::fmt::Debug + Send + Sync + 'static,
{
    type Item = T;

    fn map_status(self, status_code: StatusCode) -> StdResult<T, HttpError<R>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(E::into(e).with_status_code(status_code)),
        }
    }

    fn map_http_error<S>(
        self,
        status_code: StatusCode,
        reason: S,
    ) -> StdResult<Self::Item, HttpError<R>>
    where
        S: Into<Cow<'static, str>>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(E::into(e)
                .with_status_code(status_code)
                .with_reason(reason.into())),
        }
    }
}

/// Extension trait to transform an [`Option`] to a [`HttpError`].
pub trait OptionExt<R> {
    type Item;

    /// Transforms the `Option<T>` into a `Result<T, HttpError<R>>`, mapping `Some(v)` to
    /// `Ok(v)` and `None` to `Err(HttpError)` with status code.
    ///
    /// # Examples
    ///
    /// ```
    /// use http::StatusCode;
    /// use anyhow_http::{http_error, HttpError, OptionExt};
    ///
    /// let x: Result<_, HttpError<()>> = None::<()>.ok_or_status(StatusCode::BAD_REQUEST);
    /// assert_eq!(x, Err(http_error!(BAD_REQUEST)));
    /// ```
    fn ok_or_status(self, status_code: StatusCode) -> StdResult<Self::Item, HttpError<R>>;
}

impl<R, T> OptionExt<R> for std::option::Option<T>
where
    R: std::fmt::Debug + Send + Sync + 'static,
{
    type Item = T;

    fn ok_or_status(self, status_code: StatusCode) -> StdResult<T, HttpError<R>> {
        match self {
            Some(v) => Ok(v),
            None => Err(HttpError::from_status_code(status_code)),
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
        let http_result: StdResult<_, HttpError<()>> = result.map_status(StatusCode::BAD_REQUEST);

        let Err(e) = http_result else { unreachable!() };
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.source.unwrap().to_string(), "error".to_owned());
    }

    #[test]
    fn http_err_ext_result_map_http_error() {
        let s: StdResult<i32, HttpError<()>> =
            "nan".parse::<i32>().map_status(StatusCode::BAD_REQUEST);
        assert_eq!(s, Err(HttpError::from_status_code(StatusCode::BAD_REQUEST)));

        let result: StdResult<(), _> = Err(anyhow!("error"));
        let http_result: StdResult<_, HttpError<()>> =
            result.map_http_error(StatusCode::BAD_REQUEST, "invalid request");

        let Err(e) = http_result else { unreachable!() };
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.source.unwrap().to_string(), "error".to_owned());
        assert_eq!(e.reason, Some("invalid request".into()));
    }

    #[test]
    fn http_err_ext_option() {
        let opt: Option<()> = None;
        let http_result: StdResult<_, HttpError<()>> = opt.ok_or_status(StatusCode::BAD_REQUEST);

        let Err(e) = http_result else { unreachable!() };
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_none());
    }
}
