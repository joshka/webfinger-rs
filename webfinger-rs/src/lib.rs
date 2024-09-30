//! `webfinger-rs` is a Rust library for handling WebFinger protocol defined by [RFC 7033].
//!
//! WebFinger is  is used to discover information about people or other entities on the internet.
//! The motivation of this library is to provide a transport-agnostic implementation of the
//! WebFinger protocol for client and server-side application which can be used with different HTTP
//! libraries such as [Axum], and [Reqwest]. Additionally, the other available crates for WebFinger
//! are either not actively maintained and have a license that is incompatible with incorporating
//! the crate into other projects as a library (GPL-3.0).
//!
//! [RFC 7033]: https://www.rfc-editor.org/rfc/rfc7033.html
//! [Axum]: https://crates.io/crates/axum
//! [Reqwest]: https://crates.io/crates/reqwest
//!
//! # Usage
//!
//! To use this library, add it to your `Cargo.toml`:
//!
//! ```shell
//! cargo add webfinger-rs
//! ```
//!
//! The library also has a related CLI tool, `webfinger-cli`, which can be installed with:
//!
//! ```shell
//! cargo install webfinger-cli
//! webfinger fetch acct:carol@example.com --rel http://webfinger.net/rel/avatar
//! ```
#![doc = document_features::document_features!()]
//!
//! # Client Example
//!
//! The following example connects to the WebFinger server at `example.com` and requests the profile
//! page for the user `carol@example.com`. It requires the `reqwest` feature to be enabled. This
//! example is also available in the repository at:
//! <https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/client.rs>.
//!
//! ```rust, no_run
//! use webfinger_rs::Request;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let request = Request::builder("acct:carol@example.com")?
//!         .host("example.com")
//!         .rel("http://webfinger.net/rel/profile-page")
//!         .build();
//!     let response = request.execute().await?;
//!     dbg!(response);
//!     Ok(())
//! }
//! ```
//!
//! # Server Example
//!
//! The following example is an Axum handler that responds to WebFinger requests. It requires the
//! `axum` feature to be enabled. This example is also available in the repository at:
//! <https://gitnub.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs>.
//!
//! ```rust
//! use axum::response::Result as AxumResult;
//! use webfinger_rs::{Link, Rel, Request as WebFingerRequest, Response as WebFingerResponse};
//!
//! async fn webfinger(request: WebFingerRequest) -> AxumResult<WebFingerResponse> {
//!     let subject = request.resource.to_string();
//!     if subject != "acct:carol@example.com" {
//!         Err((http::StatusCode::NOT_FOUND, "Not Found"))?;
//!     }
//!     let rel = Rel::new("http://webfinger.net/rel/profile-page");
//!     let response = if request.rels.is_empty() || request.rels.contains(&rel) {
//!         let link = Link::builder(rel).href(format!("https://example.com/profile/{subject}"));
//!         WebFingerResponse::builder(subject).link(link).build()
//!     } else {
//!         WebFingerResponse::builder(subject).build()
//!     };
//!     Ok(response)
//! }
//! ```
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use crate::{error::Error, types::*};

#[cfg(feature = "axum")]
mod axum;
mod error;
mod http;
#[cfg(feature = "reqwest")]
mod reqwest;
mod types;

/// The well-known path for WebFinger requests (`/.well-known/webfinger`).
///
/// This is the path that should be used to query for WebFinger resources.
///
/// See [RFC 7033 Section 10.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-10.1) for more
/// information.
pub const WELL_KNOWN_PATH: &str = "/.well-known/webfinger";
