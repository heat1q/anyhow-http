# anyhow-http

<a href="https://github.com/heat1q/anyhow-http/actions/workflows/rust.yml">
<img src="https://github.com/heat1q/anyhow-http/actions/workflows/rust.yml/badge.svg" />
</a>
<a href="https://crates.io/crates/anyhow-http">
<img src="https://img.shields.io/crates/v/anyhow-http.svg" />
</a>
<a href="https://docs.rs/anyhow-http">
<img src="https://docs.rs/anyhow-http/badge.svg" />
</a>
<br/>
<br/>

`anyhow-http` offers customizable HTTP errors built on [`anyhow`](https://docs.rs/prometheus/latest/prometheus/) errors. This crates acts as a superset of [`anyhow`](https://docs.rs/prometheus/latest/prometheus/), extending the functionality to define custom HTTP error responses.

## Example 
```rust
use axum::{
   routing::get,
   response::IntoResponse,
   Router,
};
use anyhow_http::{http_error_ret, response::Result};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn fallible_operation() -> Result<()> {
    http_error_ret!(INTERNAL_SERVER_ERROR, "this is an error")
}

async fn handler() -> Result<impl IntoResponse> {
    fallible_operation()?;
    Ok(())
}
```

## License
Licensed under [MIT](https://github.com/heat1q/anyhow-http/blob/master/LICENSE).




* create http error anywhere which should be persisted -> custom Result<_, HttpError<_>>
* any error should be easily converted to HttpError -> impl From<StdError> for HttpError
* 
