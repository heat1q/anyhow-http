//! `anyhow-http` offers customizable HTTP errors built on [`anyhow`] errors.
//!
//! This crates acts as a superset of [`anyhow`], extending the functionality to define custom
//! HTTP error responses.
//!
//! # Example with `axum`
//!
//! ```rust,no_run
//! use axum::{
//!    routing::get,
//!    response::IntoResponse,
//!    Router,
//! };
//! use anyhow_http::{http_error_ret, response::Result};
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = Router::new()
//!         .route("/", get(handler));
//!
//!     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
//!     axum::Server::bind(&addr)
//!         .serve(app.into_make_service())
//!         .await
//!         .unwrap();
//! }
//!
//! fn fallible_operation() -> Result<()> {
//!     http_error_ret!(INTERNAL_SERVER_ERROR, "this is an error")
//! }
//!
//! async fn handler() -> Result<impl IntoResponse> {
//!     fallible_operation()?;
//!     Ok(())
//! }
//! ```

mod http_error;
pub use http_error::*;

#[doc(hidden)]
pub mod macros;

pub mod response;

pub use http;
