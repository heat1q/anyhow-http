/// Construct an ad-hoc [`HttpError`](super::HttpError) from a status code, optional source error and formatted reason.
///
/// ```
/// # use anyhow::anyhow;
/// # use anyhow_http::http_error;
/// fn foo() -> anyhow::Result<()> {
///     const CODE: i32 = 1234;
///     Err(http_error!(BAD_REQUEST, "invalid payload, code {}", CODE))?;
///
///     // with source
///     let source = anyhow!("source error");
///     Err(http_error!(BAD_REQUEST, source = source, reason = "invalid payload, code {}", CODE))?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! http_error{
    ($status_code:ident, $reason:literal) => {
        $crate::HttpError::from_static($crate::http::StatusCode::$status_code, $reason)
    };
    ($status_code:ident $(, source = $src:expr)? $(, reason = $($arg:tt)*)?) => {{
        let http_error
            = $crate::HttpError::from_status_code($crate::http::StatusCode::$status_code)
            $(
                .with_source_err($src)
             )?
            $(
                .with_reason(std::format!($($arg)*))
             )?;
        http_error
    }};
    ($status_code:ident $(, $($arg:tt)*)?) => {
        $crate::http_error!($status_code $(, reason = $($arg)*)?)
    };
}

/// Shorthand macro to return early with an [`HttpError`](super::HttpError).
///
/// Example:
/// ```
/// # use anyhow_http::http_error_bail;
/// fn foo() -> anyhow::Result<()> {
///     http_error_bail!(BAD_REQUEST, "invalid payload")
/// }
/// ```
#[macro_export]
macro_rules! http_error_bail {
    ($status_code:ident $(, source = $src:expr)? $(, reason = $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, source = $src)? $(, reason = $($arg)*)?).into())
    };
    ($status_code:ident $(, $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, reason = $($arg)*)?).into())
    };
}

#[cfg(test)]
mod tests {

    use crate::*;
    use anyhow::anyhow;
    use http::StatusCode;

    #[test]
    fn http_error_status_code() {
        let e: HttpError = http_error!(BAD_REQUEST);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn http_error_only_reason() {
        let i = 1;
        let e: HttpError = http_error!(BAD_REQUEST, "error {}", i);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(e.reason, Some("error 1".into()));
    }

    #[test]
    fn http_error_status_and_source() {
        let source = anyhow!("an error");
        let e: HttpError = http_error!(BAD_REQUEST, source = source);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_some());
    }

    #[test]
    fn http_error_status_source_and_format() {
        let source = anyhow!("an error");
        let e: HttpError = http_error!(BAD_REQUEST, source = source, reason = "error {}", 1);
        assert_eq!(e.status_code, StatusCode::BAD_REQUEST);
        assert!(e.source.is_some());
        assert_eq!(e.reason, Some("error 1".into()));
    }

    #[test]
    fn http_error_static() {
        const ERR: HttpError = http_error!(BAD_REQUEST, "error");
        assert_eq!(ERR.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(ERR.reason, Some("error".into()));
    }

    #[test]
    fn http_error_bridge() {
        let _err: anyhow::Error = http_error!(BAD_REQUEST, "error",).into();
        let _err: HttpError = http_error!(BAD_REQUEST, "error",);
    }
}
