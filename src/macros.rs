use std::fmt;

use crate::HttpError;

/// Construct an [`HttpError`] at compile time.
#[macro_export]
macro_rules! http_error_static {
    ($status_code:ident, $reason:literal) => {
        $crate::HttpError::from_static($crate::http::StatusCode::$status_code, $reason)
    };
}

/// Construct an ad-hoc [`HttpError`] from a status code, optional source error and formatted reason.
#[macro_export]
macro_rules! http_error{
    ($status_code:ident $(, source = $src:expr)? $(, reason = $($arg:tt)*)?) => {{
        let http_error: $crate::HttpError<()>
            = $crate::HttpError::from_status_code($crate::http::StatusCode::$status_code)
            $(
                .with_source_err($src)
             )?
            $(
                .with_reason(std::format!($($arg)*))
             )?;
        use $crate::__private::BridgeError;
        http_error.bridge_error()
    }};
    ($status_code:ident $(, $($arg:tt)*)?) => {
        $crate::http_error!($status_code $(, reason = $($arg)*)?)
    };
}

/// Shorthand macro to return early with an [`HttpError`].
#[macro_export]
macro_rules! http_error_ret {
    ($status_code:ident $(, source = $src:expr)? $(, reason = $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, source = $src)? $(, reason = $($arg)*)?))
    };
    ($status_code:ident $(, $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, reason = $($arg)*)?))
    };
}

#[doc(hidden)]
pub trait BridgeError<E> {
    fn bridge_error(self) -> E;
}

impl<R> BridgeError<anyhow::Error> for HttpError<R>
where
    R: fmt::Debug + Send + Sync + 'static,
{
    fn bridge_error(self) -> anyhow::Error {
        self.into()
    }
}

impl<R1, R2> BridgeError<HttpError<R2>> for HttpError<R1>
where
    R1: fmt::Debug,
    R2: fmt::Debug,
{
    fn bridge_error(self) -> HttpError<R2> {
        HttpError::from_http_err(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use anyhow::anyhow;
    use http::StatusCode;

    #[test]
    fn http_error_status_code() {
        let e: HttpError<()> = http_error!(BAD_REQUEST);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_only_reason() {
        let i = 1;
        let e: HttpError<()> = http_error!(BAD_REQUEST, "error {i}");
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.reason, Some("error 1".into()));
    }

    #[test]
    fn http_error_status_and_source() {
        let source = anyhow!("an error");
        let e: HttpError<()> = http_error!(BAD_REQUEST, source = source);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_some());
    }

    #[test]
    fn http_error_status_source_and_format() {
        let source = anyhow!("an error");
        let e: HttpError<()> = http_error!(BAD_REQUEST, source = source, reason = "error {}", 1);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_some());
        assert_eq!(e.reason, Some("error 1".into()));
    }

    #[test]
    fn http_error_static() {
        const ERR: HttpError<()> = http_error_static!(BAD_REQUEST, "error");
        assert_eq!(ERR.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(ERR.reason, Some("error".into()));
    }

    #[test]
    fn http_error_bridge() {
        let _err: anyhow::Error = http_error!(BAD_REQUEST, "error",);
        let _err: HttpError<i32> = http_error!(BAD_REQUEST, "error",);
        //let _err = http_error!(BAD_REQUEST, "error",);
    }
}
