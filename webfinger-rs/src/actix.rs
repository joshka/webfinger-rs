//! Actix Web integration for WebFinger request extraction and JSON responses.
//!
//! Enable the `actix` feature to:
//!
//! - extract [`WebFingerRequest`] from requests routed to [`crate::WELL_KNOWN_PATH`]; and
//! - return [`WebFingerResponse`] directly from Actix handlers.
//!
//! The extractor reads the standard WebFinger query shape from [RFC 7033 section 4.1]:
//!
//! - a required `resource` query parameter; and
//! - zero or more repeated `rel` query parameters.
//!
//! In practice, route handlers should usually be mounted like this:
//!
//! ```rust
//! use actix_web::{get, App};
//! use webfinger_rs::{WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
//!
//! #[get("/.well-known/webfinger")]
//! async fn webfinger(_request: WebFingerRequest) -> WebFingerResponse {
//!     WebFingerResponse::new("acct:carol@example.com")
//! }
//!
//! let app = App::new().service(webfinger);
//! # let _ = app;
//! # assert_eq!(WELL_KNOWN_PATH, "/.well-known/webfinger");
//! ```
//!
//! If extraction fails, Actix currently returns `400 Bad Request` for missing `resource` or
//! missing host values. Malformed `resource` values are not currently mapped into an Actix error;
//! they panic during extraction because request construction uses
//! [`WebFingerRequest::builder`][crate::WebFingerRequest::builder] with `unwrap()`.
//!
//! See also [`WebFingerRequest`] for the extractor impl, [`WebFingerResponse`] for the responder
//! impl, and the [Actix example] for a runnable server.
//!
//! [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
//! [Actix example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/actix.rs

use std::future::Future;
use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::web::Json;
use actix_web::{FromRequest, HttpRequest, HttpResponse, Responder};
use tracing::trace;

use crate::{WebFingerRequest, WebFingerResponse};

impl Responder for WebFingerResponse {
    /// Converts a [`WebFingerResponse`] into an Actix response.
    ///
    /// This delegates to [`actix_web::web::Json`], so the body is serialized as JSON and the
    /// response `Content-Type` follows Actix's JSON responder behavior, which is currently
    /// `application/json`.
    ///
    /// Unlike the Axum integration, this responder does not currently override the content type to
    /// `application/jrd+json`.
    ///
    /// See also the [`crate::actix`] module docs and the [Actix example].
    ///
    /// # Example
    ///
    /// ```rust
    /// use actix_web::{get, App};
    /// use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse};
    ///
    /// #[get("/.well-known/webfinger")]
    /// async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
    ///     let subject = request.resource.to_string();
    ///     let rel = Rel::new("http://webfinger.net/rel/profile-page");
    ///     let response = if request.rels.is_empty() || request.rels.contains(&rel) {
    ///         let link = Link::builder(rel).href("https://example.com/users/carol");
    ///         WebFingerResponse::builder(subject).link(link).build()
    ///     } else {
    ///         WebFingerResponse::builder(subject).build()
    ///     };
    ///     Ok(response)
    /// }
    ///
    /// let app = App::new().service(webfinger);
    /// # let _ = app;
    /// ```
    ///
    /// [Actix example]:
    ///     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/actix.rs
    type Body = <Json<WebFingerResponse> as Responder>::Body;

    fn respond_to(self, _request: &HttpRequest) -> HttpResponse<Self::Body> {
        Json(self).respond_to(_request)
    }
}

impl FromRequest for WebFingerRequest {
    /// Extracts a [`WebFingerRequest`] from an Actix request.
    ///
    /// The extractor reads:
    ///
    /// - the host from the request URI authority or the HTTP `Host` header;
    /// - the `resource` query parameter from the raw query string; and
    /// - every repeated `rel` query parameter from that same raw query string.
    ///
    /// The query parsing is intentionally simple and follows the current implementation rather than
    /// Actix's typed query extractor.
    ///
    /// # Errors
    ///
    /// - If the request has no `resource` query parameter, extraction returns
    ///   `ErrorBadRequest("missing resource")`.
    /// - If the request has no URI authority and no `Host` header, extraction returns
    ///   `ErrorBadRequest("missing host")`.
    /// - If `resource` is present but cannot be parsed by [`WebFingerRequest::builder`], the
    ///   current implementation panics instead of returning an Actix error.
    ///
    /// See also the [`crate::actix`] module docs and the [Actix example].
    ///
    /// # Example
    ///
    /// ```rust
    /// use actix_web::{get, App};
    /// use webfinger_rs::{WebFingerRequest, WebFingerResponse};
    ///
    /// #[get("/.well-known/webfinger")]
    /// async fn webfinger(request: WebFingerRequest) -> WebFingerResponse {
    ///     WebFingerResponse::new(request.resource.to_string())
    /// }
    ///
    /// let app = App::new().service(webfinger);
    /// # let _ = app;
    /// ```
    ///
    /// [Actix example]:
    ///     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/actix.rs
    type Error = actix_web::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        trace!(?req, "extracting WebFingerRequest from request");
        let host = req
            .uri()
            .host()
            .or_else(|| req.headers().get("host").and_then(|h| h.to_str().ok()))
            .map(|h| h.to_string());
        let resource = req
            .query_string()
            .split('&')
            .find_map(|param| param.split_once('=').filter(|(key, _)| *key == "resource"))
            .map(|(_, value)| value.to_string());
        let rels_from_query: Vec<_> = req
            .query_string()
            .split('&')
            .filter_map(|param| param.split_once('=').filter(|(key, _)| *key == "rel"))
            .map(|(_, value)| value.to_string())
            .collect();
        Box::pin(async move {
            let resource = resource.ok_or(actix_web::error::ErrorBadRequest("missing resource"))?;
            let host = host.ok_or(actix_web::error::ErrorBadRequest("missing host"))?;
            let mut request_builder = WebFingerRequest::builder(resource).unwrap().host(host);
            for rel in rels_from_query {
                request_builder = request_builder.rel(rel);
            }
            Ok(request_builder.build())
        })
    }
}
