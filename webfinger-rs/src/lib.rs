//! `webfinger-rs` is a transport-agnostic [WebFinger] implementation for Rust, centered on the
//! request and response types defined by [RFC 7033] with first-party integrations for [Reqwest],
//! [Axum], and [Actix Web].
//!
//! WebFinger is used to discover information about people or other entities on the internet using
//! URI-based identifiers such as `acct:carol@example.com`. In practice, it is commonly used for
//! [OpenID Connect Discovery], account discovery in federated systems like [Mastodon] and
//! [ActivityPub], and for publishing identity-related metadata from your own site or service.
//!
//! This crate exists to provide one set of WebFinger types that can be reused across clients,
//! servers, and tests instead of reimplementing the protocol details for each framework. It is
//! intended to be practical as both a library dependency and an integration layer for modern Rust
//! web stacks.
//!
//! It also fills a gap left by older WebFinger crates that are no longer actively maintained or
//! are licensed in a way that is less convenient for reuse as a general-purpose Rust library.
//!
//! # Why use `webfinger-rs`?
//!
//! - Reusable request and response types shaped around RFC 7033.
//! - Optional Reqwest client execution via [`WebFingerRequest::execute_reqwest`].
//! - Optional Axum and Actix Web extractor/responder integrations.
//! - A permissive dual license (`MIT OR Apache-2.0`) that fits typical library and application
//!   usage.
//!
//! [RFC 7033]: https://www.rfc-editor.org/rfc/rfc7033.html
//! [WebFinger]: https://en.wikipedia.org/wiki/WebFinger
//! [Reqwest]: https://crates.io/crates/reqwest
//! [Axum]: https://crates.io/crates/axum
//! [Actix Web]: https://crates.io/crates/actix-web
//! [OpenID Connect Discovery]: https://openid.net/specs/openid-connect-discovery-1_0.html
//! [Mastodon]: https://docs.joinmastodon.org/spec/webfinger/
//! [ActivityPub]: https://www.w3.org/TR/activitypub/
//! [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
//! [RFC 7033 section 4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4
//! [RFC 7033 section 10.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-10.1
//!
//! # Install
//!
//! Start with the core crate, then enable the integration feature you need:
//!
//! ```shell
//! cargo add webfinger-rs
//! cargo add webfinger-rs --features reqwest
//! cargo add webfinger-rs --features axum
//! cargo add webfinger-rs --features actix
//! ```
//!
//! The related CLI tool, [`webfinger-cli`], is useful for trying servers by hand:
//!
//! ```shell
//! cargo install webfinger-cli
//! webfinger acct:carol@example.com --rel http://webfinger.net/rel/avatar
//! ```
//!
//! [`webfinger-cli`]: https://crates.io/crates/webfinger-cli
//!
//! # Feature matrix
//!
//! | Feature | What it enables |
//! | --- | --- |
//! | none | Core request/response types, builders, and URL conversion support |
//! | `reqwest` | Client execution helpers and Reqwest request/response conversions |
//! | `axum` | [`WebFingerRequest`] extraction and [`WebFingerResponse`] responses in Axum via [`crate::axum`] |
//! | `actix` | [`WebFingerRequest`] extraction and [`WebFingerResponse`] responses in Actix Web |
//!
//! # Protocol overview
//!
//! A WebFinger query is an HTTPS `GET` against the well-known endpoint
//! [`WELL_KNOWN_PATH`] with a required `resource` parameter and, optionally, one or more `rel`
//! parameters.
//!
//! A request built by this crate today for `acct:carol@example.com` filtered to the profile-page
//! relation looks like this:
//!
//! ```text
//! GET https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://webfinger.net/rel/profile-page
//! ```
//!
//! See: [RFC 7033 section 4.1] for the query-construction rules and percent-encoding details.
//!
//! A successful JRD response might look like this:
//!
//! ```json
//! {
//!   "subject": "acct:carol@example.com",
//!   "links": [
//!     {
//!       "rel": "http://webfinger.net/rel/profile-page",
//!       "href": "https://example.com/users/carol"
//!     }
//!   ]
//! }
//! ```
//!
//! See: [RFC 7033 section 4.4] for the JRD structure.
//!
//! # Client quickstart
//!
//! Enable the `reqwest` feature to execute WebFinger requests directly from the request type.
//! The current API expects an explicit host, which should normally match the resource host when the
//! resource URI has one.
//!
//! ```rust,no_run
//! # #[cfg(feature = "reqwest")] {
//! use webfinger_rs::WebFingerRequest;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let request = WebFingerRequest::builder("acct:carol@example.com")?
//!         .host("example.com")
//!         .rel("http://webfinger.net/rel/profile-page")
//!         .build();
//!
//!     let response = request.execute_reqwest().await?;
//!     println!("{response:#?}");
//!     Ok(())
//! }
//! # }
//! ```
//!
//! # Axum quickstart
//!
//! Enable the `axum` feature to extract [`WebFingerRequest`] from the incoming request and return
//! [`WebFingerResponse`] directly from your handler. Mount the handler at [`WELL_KNOWN_PATH`].
//! See also [`crate::axum`] and the [Axum example].
//!
//! ```rust
//! # #[cfg(feature = "axum")]
//! # fn app() -> axum::Router {
//! use axum::{http::StatusCode, routing::get, Router};
//! use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
//!
//! async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
//!     let subject = request.resource.to_string();
//!     if subject != "acct:carol@example.com" {
//!         return Err((StatusCode::NOT_FOUND, "not found").into());
//!     }
//!
//!     let rel = Rel::new("http://webfinger.net/rel/profile-page");
//!     let response = if request.rels.is_empty() || request.rels.contains(&rel) {
//!         let link = Link::builder(rel).href("https://example.com/users/carol");
//!         WebFingerResponse::builder(subject).link(link).build()
//!     } else {
//!         WebFingerResponse::builder(subject).build()
//!     };
//!     Ok(response)
//! }
//!
//! Router::new().route(WELL_KNOWN_PATH, get(webfinger))
//! # }
//! ```
//!
//! [Axum example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
//!
//! # Actix quickstart
//!
//! Enable the `actix` feature to use the same request and response types in Actix Web handlers.
//! As with the Axum integration, the route path should be [`WELL_KNOWN_PATH`].
//!
//! ```rust
//! # #[cfg(feature = "actix")]
//! # fn app() -> actix_web::App<
//! #     impl actix_web::dev::ServiceFactory<
//! #         actix_web::dev::ServiceRequest,
//! #         Config = (),
//! #         Response = actix_web::dev::ServiceResponse,
//! #         Error = actix_web::Error,
//! #         InitError = (),
//! #     >,
//! # > {
//! use actix_web::{get, App};
//! use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse};
//!
//! #[get("/.well-known/webfinger")]
//! async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
//!     let subject = request.resource.to_string();
//!     if subject != "acct:carol@example.com" {
//!         return Err(actix_web::error::ErrorNotFound("not found"));
//!     }
//!
//!     let rel = Rel::new("http://webfinger.net/rel/profile-page");
//!     let response = if request.rels.is_empty() || request.rels.contains(&rel) {
//!         let link = Link::builder(rel).href("https://example.com/users/carol");
//!         WebFingerResponse::builder(subject).link(link).build()
//!     } else {
//!         WebFingerResponse::builder(subject).build()
//!     };
//!     Ok(response)
//! }
//!
//! App::new().service(webfinger)
//! # }
//! ```
//!
//! # Compatibility
//!
//! The current first-party integration targets are:
//!
//! - Reqwest `0.13`
//! - Axum `0.8`
//! - Actix Web `4`
//!
//! The crate is currently pre-`0.1`, so API and compatibility adjustments may still land in minor
//! releases while the integration surface settles. These version notes describe the currently
//! integrated crates, not a full protocol-compliance matrix.
//!
//! # Limitations
//!
//! - Client execution is currently implemented only for Reqwest.
//! - Server integrations are currently implemented only for Axum and Actix Web.
//! - The crate focuses on RFC 7033 request/response handling and framework integration, not a full
//!   identity stack around WebFinger.
//! - The crate docs aim to stay grounded in RFC 7033, but they document the current implementation
//!   rather than exhaustively enumerating every compliance detail.
//!
//! See: [RFC 7033 section 10.1] for the well-known path registration.
//!
//! # Examples
//!
//! Runnable examples are available in the repository:
//!
//! - `cargo run --example client --features reqwest`
//! - `cargo run --example axum --features axum`
//! - `cargo run --example actix --features actix`
//!
//! The server examples listen on `https://localhost:3000` and can be queried with:
//!
//! ```shell
//! webfinger acct:carol@localhost localhost:3000 --insecure --rel http://webfinger.net/rel/profile-page
//! ```
//!
//! # License
//!
//! Copyright (c) Josh McKinney
//!
//! This project is licensed under either of:
//!
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
//!   <https://apache.org/licenses/LICENSE-2.0>)
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>) at your
//!   option
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use crate::error::Error;
pub use crate::types::{
    Link, LinkBuilder, Rel, Request as WebFingerRequest, RequestBuilder,
    Response as WebFingerResponse, ResponseBuilder, Title,
};

#[cfg(feature = "actix")]
mod actix;
#[cfg(feature = "axum")]
pub mod axum;
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
