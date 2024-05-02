//! `anyhow-http` offers customizable HTTP errors built on [`anyhow`] errors.
//!
//! This crates acts as a superset of [`anyhow`], extending the functionality to define custom
//! HTTP error responses.
//! # Example with `axum`
//!
//! ```rust,no_run
//! use axum::{
//!    routing::get,
//!    response::IntoResponse,
//!    Router,
//! };
//! use anyhow_http::{http_error_ret, response::HttpJsonResult};
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = Router::new()
//!         .route("/", get(handler));
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//!         .await
//!         .unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//!
//! fn fallible_operation() -> anyhow::Result<()> {
//!     http_error_ret!(INTERNAL_SERVER_ERROR, "this is an error")
//! }
//!
//! async fn handler() -> HttpJsonResult<impl IntoResponse> {
//!     fallible_operation()?;
//!     Ok(())
//! }
//! ```

mod extension;
mod http_error;

pub use extension::*;
pub use http_error::*;

#[doc(hidden)]
pub mod macros;

pub mod response;

pub use http;

//// Not public API.
//#[doc(hidden)]
//pub mod __private {
//    pub use crate::macros::BridgeError;
//}
