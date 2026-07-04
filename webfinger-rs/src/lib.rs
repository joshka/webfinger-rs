//! `webfinger-rs` is a transport-agnostic [WebFinger] implementation for Rust, centered on the
//! request and response types defined by [RFC 7033] with first-party integrations for [Reqwest],
//! [Axum], and [Actix Web].
//!
//! WebFinger is used to discover information about people or other entities on the internet using
//! URI-based identifiers such as `acct:carol@example.com`. In practice, it is commonly used for
//! [OpenID Connect Discovery], account discovery in federated systems like [Mastodon] and
//! [ActivityPub], and for publishing identity-related metadata from your own site or service.
//!
//! The crate keeps request parsing, JRD response construction, and framework adapters in one place
//! so clients, servers, and tests use the same WebFinger types.
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
//! | none | Core request/response types, builders, and URL conversion |
//! | `reqwest` | Client execution helpers and Reqwest request/response conversions |
//! | `axum` | [`WebFingerRequest`] extraction and [`WebFingerResponse`] responses in Axum via [`webfinger_rs::axum`] |
//! | `actix` | [`WebFingerRequest`] extraction and [`WebFingerResponse`] responses in Actix Web via [`webfinger_rs::actix`] |
//!
//! # Primary types
//!
//! - [`WebFingerRequest`] models the WebFinger query target, host, and optional relation filters.
//!   Build one directly for client requests, or extract one from an Axum or Actix handler.
//! - [`WebFingerResponse`] models the JSON Resource Descriptor returned by a WebFinger endpoint.
//!   Return one from server handlers or parse one from a Reqwest response.
//! - [`Link`] and [`Rel`] model JRD link objects and relation filters so servers can apply the
//!   same relation-filtering rules that clients request.
//! - [`Resource`] and [`JrdUri`] validate URI-valued protocol fields before they enter requests or
//!   JRD responses.
//!
//! # Protocol overview
//!
//! A WebFinger query is an HTTPS `GET` against the well-known endpoint
//! [`WELL_KNOWN_PATH`] with a required `resource` parameter and, optionally, one or more `rel`
//! parameters. The `resource` parameter is the query target URI; builders and server extractors
//! reject relative references such as `carol`, `/relative`, `../x`, and empty values.
//!
//! A request built by this crate today for `acct:carol@example.com` filtered to the profile-page
//! relation looks like this:
//!
//! ```text
//! GET https://example.com/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fwebfinger.net%2Frel%2Fprofile-page
//! ```
//!
//! See: [RFC 7033 section 4.1] for the query-construction rules and percent-encoding details.
//!
//! Server integrations leave routing and TLS at the framework boundary, then use WebFinger
//! extractors for protocol parsing:
//!
//! - mount the handler as `GET` at [`WELL_KNOWN_PATH`] so the router rejects other paths and
//!   methods;
//! - configure TLS and forwarded-proto handling at the server or reverse-proxy boundary; and
//! - let the [`webfinger_rs::axum`] or [`webfinger_rs::actix`] extractor validate the request host,
//!   query parameters, percent encoding, and `resource` URI.
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
//! const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
//! const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let request = WebFingerRequest::builder("acct:carol@example.com")?
//!         .host("example.com")
//!         .rel(PROFILE_PAGE_REL)
//!         .rel(AVATAR_REL)
//!         .build();
//!
//!     let response = request.execute_reqwest().await?;
//!     println!("Subject: {}", response.subject);
//!     for rel in [PROFILE_PAGE_REL, AVATAR_REL] {
//!         if let Some(href) = response
//!             .links
//!             .iter()
//!             .find(|link| link.rel.as_ref() == rel)
//!             .and_then(|link| link.href.as_ref().map(|href| href.as_ref()))
//!         {
//!             println!("{rel}: {href}");
//!         }
//!     }
//!     println!("{response}");
//!     Ok(())
//! }
//! # }
//! ```
//!
//! # Axum quickstart
//!
//! Enable the `axum` feature to extract [`WebFingerRequest`] from the incoming request and return
//! [`WebFingerResponse`] directly from your handler. Mount the handler at [`WELL_KNOWN_PATH`].
//! See also [`webfinger_rs::axum`] and the [Axum example].
//!
//! ```rust
//! # #[cfg(feature = "axum")]
//! # fn app() -> axum::Router {
//! use axum::{http::StatusCode, routing::get, Router};
//! use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
//!
//! const SUBJECT: &str = "acct:carol@example.com";
//! const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
//! const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
//! const PROFILE_URL: &str = "https://example.com/users/carol";
//! const AVATAR_URL: &str = "https://example.com/media/carol.png";
//! const ROLE_PROPERTY: &str = "https://example.com/ns/account-role";
//!
//! async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
//!     let subject = request.resource.to_string();
//!     if subject != SUBJECT {
//!         return Err((StatusCode::NOT_FOUND, "not found").into());
//!     }
//!
//!     let mut links = Vec::new();
//!
//!     let profile_rel = Rel::new(PROFILE_PAGE_REL);
//!     if request.rels.is_empty() || request.rels.contains(&profile_rel) {
//!         links.push(
//!             Link::builder(profile_rel)
//!                 .href(PROFILE_URL)
//!                 .title("en", "Carol's profile")
//!                 .build(),
//!         );
//!     }
//!
//!     let avatar_rel = Rel::new(AVATAR_REL);
//!     if request.rels.is_empty() || request.rels.contains(&avatar_rel) {
//!         links.push(
//!             Link::builder(avatar_rel)
//!                 .href(AVATAR_URL)
//!                 .r#type("image/png")
//!                 .build(),
//!         );
//!     }
//!
//!     let response = WebFingerResponse::builder(subject)
//!         .alias(PROFILE_URL)
//!         .property(ROLE_PROPERTY, "maintainer")
//!         .links(links)
//!         .build();
//!     Ok(response)
//! }
//!
//! Router::new().route(WELL_KNOWN_PATH, get(webfinger))
//! # }
//! ```
//!
//! [Axum example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
//! [`webfinger_rs::axum`]: https://docs.rs/webfinger-rs/latest/webfinger_rs/axum/
//!
//! # Actix quickstart
//!
//! Enable the `actix` feature to use the same request and response types in Actix Web handlers.
//! As with the Axum integration, the route path should be [`WELL_KNOWN_PATH`]. See also
//! [`webfinger_rs::actix`] and the [Actix example].
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
//! use actix_web::{App, web};
//! use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
//!
//! const SUBJECT: &str = "acct:carol@example.com";
//! const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
//! const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
//! const PROFILE_URL: &str = "https://example.com/users/carol";
//! const AVATAR_URL: &str = "https://example.com/media/carol.png";
//! const ROLE_PROPERTY: &str = "https://example.com/ns/account-role";
//!
//! async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
//!     let subject = request.resource.to_string();
//!     if subject != SUBJECT {
//!         return Err(actix_web::error::ErrorNotFound("not found"));
//!     }
//!
//!     let mut links = Vec::new();
//!
//!     let profile_rel = Rel::new(PROFILE_PAGE_REL);
//!     if request.rels.is_empty() || request.rels.contains(&profile_rel) {
//!         links.push(
//!             Link::builder(profile_rel)
//!                 .href(PROFILE_URL)
//!                 .title("en", "Carol's profile")
//!                 .build(),
//!         );
//!     }
//!
//!     let avatar_rel = Rel::new(AVATAR_REL);
//!     if request.rels.is_empty() || request.rels.contains(&avatar_rel) {
//!         links.push(
//!             Link::builder(avatar_rel)
//!                 .href(AVATAR_URL)
//!                 .r#type("image/png")
//!                 .build(),
//!         );
//!     }
//!
//!     let response = WebFingerResponse::builder(subject)
//!         .alias(PROFILE_URL)
//!         .property(ROLE_PROPERTY, "maintainer")
//!         .links(links)
//!         .build();
//!     Ok(response)
//! }
//!
//! App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger))
//! # }
//! ```
//!
//! [Actix example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/actix.rs
//! [`webfinger_rs::actix`]: https://docs.rs/webfinger-rs/latest/webfinger_rs/actix/
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
//! - `cargo run -p webfinger-rs --example axum --features axum`
//! - `cargo run -p webfinger-rs --example actix --features actix`
//! - `cargo run -p webfinger-rs --example client --features reqwest`
//!
//! Run one server example first, then run the client example in another shell. The client example
//! queries `https://localhost:3000`, accepts the self-signed certificate generated by either server
//! example, and prints the profile-page and avatar links returned by the shared
//! [`WebFingerResponse`] type.
//!
//! The server examples also work with the CLI. Query without `--rel` to get both links, or pass a
//! relation filter to narrow the returned `links` array:
//!
//! ```shell
//! webfinger acct:carol@localhost localhost:3000 --insecure
//! webfinger acct:carol@localhost localhost:3000 --insecure --rel http://webfinger.net/rel/profile-page
//! webfinger acct:carol@localhost localhost:3000 --insecure --rel http://webfinger.net/rel/avatar
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
    JrdUri, Link, LinkBuilder, Rel, Request as WebFingerRequest, RequestBuilder, Resource,
    ResourceError, Response as WebFingerResponse, ResponseBuilder, Title,
};

#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "axum")]
pub mod axum;
mod error;
mod http;
#[cfg(any(feature = "actix", feature = "axum", test))]
mod query;
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
