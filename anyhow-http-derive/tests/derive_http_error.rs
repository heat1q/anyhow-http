use anyhow_http::{http_error, HttpError};
use anyhow_http_derive::FromHttpError;

const DATA_KEY: &str = "key";

#[derive(Debug, FromHttpError)]
enum CustomError {
    #[http_error(status(400), reason("reason {0}"))]
    From(#[from] anyhow::Error),
    #[http_error(status(400), reason("reason {count}"))]
    NamedWithSource {
        count: u64,
        #[source]
        source: anyhow::Error,
    },
    #[http_error(
        status(400),
        reason("reason {0}"),
        data(code = 1234, info = "info {0}")
    )]
    UnamedWithSource(u64, #[source] anyhow::Error),
    #[http_error(transparent)]
    Transparent(#[source] HttpError),
    #[http_error(status(404), data({DATA_KEY} = ErrorCode::Invalid as u32))]
    ExprData(String),
}

#[derive(Debug)]
#[repr(u32)]
enum ErrorCode {
    Invalid = 1234,
}

#[test]
fn derive_enum_from() {
    let res: Result<(), CustomError> = (|| {
        Err(anyhow::anyhow!("source"))?;
        unreachable!()
    })();
    let err: HttpError = res.unwrap_err().into();

    assert_eq!(err.status_code(), 400);
    assert_eq!(err.reason(), Some("reason source".into()));
    assert_eq!(err.source().map(ToString::to_string), Some("source".into()));
}

#[test]
fn derive_enum_named_with_source() {
    let err: HttpError = CustomError::NamedWithSource {
        source: anyhow::anyhow!("source"),
        count: 1234,
    }
    .into();

    assert_eq!(err.status_code(), 400);
    assert_eq!(err.reason(), Some("reason 1234".into()));
    assert_eq!(err.source().map(ToString::to_string), Some("source".into()));
}

#[test]
fn derive_enum_unnamed_with_source() {
    let err: HttpError = CustomError::UnamedWithSource(1234, anyhow::anyhow!("source")).into();

    assert_eq!(err.status_code(), 400);
    assert_eq!(err.reason(), Some("reason 1234".into()));
    assert_eq!(err.get("code"), Some(1234));
    assert_eq!(err.get("info"), Some("info 1234".to_string()));
    assert_eq!(err.source().map(ToString::to_string), Some("source".into()));
}

#[test]
fn derive_enum_transparent() {
    let err: HttpError = CustomError::Transparent(http_error!(BAD_REQUEST, "bad request")).into();

    assert_eq!(err.status_code(), 400);
    assert_eq!(err.reason(), Some("bad request".into()));
    assert!(err.source().is_none());
}

#[test]
fn derive_enum_expr_data() {
    let err: HttpError = CustomError::ExprData("expr data".into()).into();

    assert_eq!(err.status_code(), 404);
    assert_eq!(err.get(DATA_KEY), Some(1234));
    assert_eq!(err.reason(), None);
    assert_eq!(
        err.source().map(ToString::to_string),
        Some("CustomError :: ExprData".into())
    );
}
