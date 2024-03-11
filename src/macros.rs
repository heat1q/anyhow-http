/// Construct an ad-hoc `HttpError` from a status code, optional source error and formatted reason.
#[macro_export]
macro_rules! http_error{
    ($status_code:ident, $reason:literal) => {
       $crate::HttpError::from_static($crate::http::StatusCode::$status_code, $reason)
    };
    ($status_code:ident $(, source = $src:ident)? $(, reason = $($arg:tt)*)?) => {{
        let http_error: $crate::HttpError<_>
            = $crate::HttpError::from_status_code($crate::http::StatusCode::$status_code)
            $(
                .with_source_err($src)
             )?
            $(
                .with_reason(std::format!($($arg)*))
             )?;
        http_error
    }};
    ($status_code:ident $(, $($arg:tt)*)?) => {{
        let http_error: $crate::HttpError<_>
            = $crate::HttpError::from_status_code($crate::http::StatusCode::$status_code)
            $(
                .with_reason(std::format!($($arg)*))
             )?;
        http_error
    }};
}

/// Shorthand macro to return early with a `HttpError`.
#[macro_export]
macro_rules! http_error_ret {
    ($status_code:ident $(, source = $src:ident)? $(, reason = $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, source = $src)? $(, reason = $($arg)*)?).into())
    };
    ($status_code:ident $(, $($arg:tt)*)?) => {
        return Err($crate::http_error!($status_code $(, $($arg)*)?).into())
    };
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
        let e: HttpError<()> = http_error!(BAD_REQUEST, "error {}", 1);
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
    fn http_error_const() {
        const ERR: HttpError<()> = http_error!(BAD_REQUEST, "error");
        assert_eq!(ERR.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(ERR.reason, Some("error".into()));
    }
}
