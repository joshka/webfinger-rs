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
//! If extraction fails, Actix returns `400 Bad Request` for missing or duplicated `resource`,
//! missing host values, invalid percent encoding, or invalid resource URIs.
//!
//! See also [`WebFingerRequest`] for the extractor impl, [`WebFingerResponse`] for the responder
//! impl, and the [Actix example] for a runnable server.
//!
//! [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
//! [Actix example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/actix.rs

use std::future::{Ready, ready};

use actix_web::dev::Payload;
use actix_web::error::ErrorBadRequest;
use actix_web::web::Json;
use actix_web::{Error as ActixError, FromRequest, HttpRequest, HttpResponse, Responder};
use tracing::trace;

use crate::query::{RequestParams, RequestParamsError};
use crate::{Rel, WebFingerRequest, WebFingerResponse};

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
    /// - the decoded `resource` query parameter; and
    /// - every repeated decoded `rel` query parameter.
    ///
    /// Query parsing percent-decodes parameters while preserving RFC 3986 query semantics.
    ///
    /// # Errors
    ///
    /// - If the request has zero or more than one `resource` query parameter, extraction returns a
    ///   bad request.
    /// - If the request has no URI authority and no `Host` header, extraction returns
    ///   `ErrorBadRequest("missing host")`.
    /// - If the query contains malformed percent encoding, extraction returns a bad request.
    /// - If `resource` is present but cannot be parsed as a URI, extraction returns
    ///   `ErrorBadRequest("invalid resource: ...")`.
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
    type Error = ActixError;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        trace!(?req, "extracting WebFingerRequest from request");
        ready(extract_request(req))
    }
}

/// Extracts WebFinger request data from Actix request metadata.
///
/// WebFinger request extraction only needs URI, host, and query metadata from RFC 7033 sections 4.1
/// and 4.2, so the fallible work can stay synchronous and the Actix [`FromRequest`] implementation
/// can wrap the result in a ready future.
fn extract_request(req: &HttpRequest) -> Result<WebFingerRequest, ActixError> {
    let host = req
        .uri()
        .host()
        .or_else(|| req.headers().get("host").and_then(|h| h.to_str().ok()))
        .map(|h| h.to_string())
        .ok_or(ErrorBadRequest("missing host"))?;
    let query = req.query_string().parse::<RequestParams>()?;
    let resource = query
        .resource
        .parse()
        .map_err(|error| ErrorBadRequest(format!("invalid resource: {error}")))?;
    let rels = query.rel.into_iter().map(Rel::from).collect();
    Ok(WebFingerRequest {
        host,
        resource,
        rels,
    })
}

impl From<RequestParamsError> for ActixError {
    fn from(error: RequestParamsError) -> Self {
        ErrorBadRequest(error)
    }
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};

    use super::*;
    use crate::WELL_KNOWN_PATH;

    type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    /// Returns the extracted resource so tests can assert RFC 7033 query decoding behavior.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    async fn webfinger(request: WebFingerRequest) -> HttpResponse {
        HttpResponse::Ok().body(request.resource.to_string())
    }

    /// Returns extracted relation filters so tests can assert RFC 7033 repeated `rel` handling.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.3>.
    async fn webfinger_rels(request: WebFingerRequest) -> HttpResponse {
        let rels = request
            .rels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        HttpResponse::Ok().json(rels)
    }

    /// Accepts a percent-encoded `acct:` resource without panicking.
    ///
    /// The resource query value is percent-encoded under RFC 7033 section 4.1, then parsed as a
    /// URI query target under RFC 7033 section 4.2.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[actix_web::test]
    async fn valid_percent_encoded_resource() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let uri = format!("{WELL_KNOWN_PATH}?resource=acct%3Abad%40example.org");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"acct:bad@example.org");
        Ok(())
    }

    /// Converts malformed resource values into Actix bad-request responses.
    ///
    /// RFC 7033 section 4.2 requires absent or malformed `resource` parameters to be treated as bad
    /// requests instead of panicking inside extraction.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[actix_web::test]
    async fn request_with_invalid_resource() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let uri = format!("{WELL_KNOWN_PATH}?resource=http%3A%2F%2F%5B%3A%3A1");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"invalid resource: invalid authority");
        Ok(())
    }

    /// Preserves repeated `rel` parameters instead of collapsing them.
    ///
    /// WebFinger clients use repeated `rel` keys to request multiple relation filters. A generic
    /// map-shaped query parser can easily keep only one value, which would make handlers see an
    /// incomplete request.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[actix_web::test]
    async fn valid_request_with_repeated_rel_params() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger_rels));
        let app = test::init_service(app).await;
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("{WELL_KNOWN_PATH}?resource={resource}&rel=profile&rel=avatar");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), br#"["profile","avatar"]"#);
        Ok(())
    }

    /// Exposes decoded relation URIs to Actix handlers.
    ///
    /// The shared parser owns the RFC 3986 percent-decoding rule; this adapter test proves Actix
    /// handlers receive decoded `Rel` values rather than raw query text.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[actix_web::test]
    async fn rel_params_are_percent_decoded() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger_rels));
        let app = test::init_service(app).await;
        let resource = "acct%3Acarol%40example.org";
        let rel = "http%3A%2F%2Fwebfinger.example%2Frel%2Fprofile-page";
        let uri = format!("{WELL_KNOWN_PATH}?resource={resource}&rel={rel}");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(
            body.as_ref(),
            br#"["http://webfinger.example/rel/profile-page"]"#,
        );
        Ok(())
    }

    /// Converts invalid UTF-8 after percent decoding into an Actix bad-request response.
    ///
    /// The shared parser owns the byte-level validation; this adapter test proves malformed
    /// percent-encoded bytes do not reach an Actix handler as relation strings.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[actix_web::test]
    async fn invalid_percent_encoded_rel_is_bad_request() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger_rels));
        let app = test::init_service(app).await;
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("{WELL_KNOWN_PATH}?resource={resource}&rel=%FF");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"invalid percent-encoded query parameter");
        Ok(())
    }

    /// Rejects malformed percent escape syntax instead of treating `%` literally.
    ///
    /// The shared query parser owns the RFC 3986 check; this Actix test proves that parser errors are
    /// converted into `400 Bad Request` responses instead of escaping the extractor boundary.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[actix_web::test]
    async fn malformed_percent_escape_is_bad_request() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger_rels));
        let app = test::init_service(app).await;
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("{WELL_KNOWN_PATH}?resource={resource}&rel=%GG");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"invalid percent-encoded query parameter");
        Ok(())
    }

    /// Accepts `resource` in any query parameter position through the Actix extractor.
    ///
    /// RFC 7033 section 4.1 does not make parameter order significant. This adapter test proves
    /// Actix handlers still receive relation filters when `resource` appears after them.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[actix_web::test]
    async fn resource_parameter_order_does_not_matter() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger_rels));
        let app = test::init_service(app).await;
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("{WELL_KNOWN_PATH}?rel=profile&resource={resource}");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), br#"["profile"]"#);
        Ok(())
    }

    /// Keeps encoded `=` and `&` inside handler-visible resource values.
    ///
    /// Resource URIs may contain query strings of their own. This adapter test proves Actix receives
    /// the decoded target resource without splitting encoded inner delimiters into WebFinger
    /// parameters.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[actix_web::test]
    async fn encoded_delimiters_stay_inside_resource() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let resource = "https%3A%2F%2Fexample.org%2Fprofile%3Fa%3D1%26b%3D2";
        let uri = format!("{WELL_KNOWN_PATH}?resource={resource}");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"https://example.org/profile?a=1&b=2");
        Ok(())
    }

    /// Preserves literal `+` in Actix handler-visible resources.
    ///
    /// Actix's normal query extractor is not used here because WebFinger follows RFC 3986 query
    /// semantics, where `+` remains data instead of becoming a space.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.4>.
    #[actix_web::test]
    async fn plus_is_not_decoded_as_space() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let uri = format!("{WELL_KNOWN_PATH}?resource=acct%3Acarol+tag%40example.org");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"acct:carol+tag@example.org");
        Ok(())
    }

    /// Rejects duplicate `resource` parameters at the Actix extractor boundary.
    ///
    /// The parser owns the RFC 7033 section 4.2 rule that there is exactly one target. This adapter
    /// test proves ambiguous requests become `400 Bad Request` responses rather than arbitrary
    /// handler inputs.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[actix_web::test]
    async fn request_with_multiple_resources() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let carol = "acct%3Acarol%40example.org";
        let alice = "acct%3Aalice%40example.org";
        let uri = format!("{WELL_KNOWN_PATH}?resource={carol}&resource={alice}");
        let request = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"multiple resource parameters");
        Ok(())
    }

    /// Rejects requests that omit the required `resource` parameter.
    ///
    /// The shared query parser owns the exact RFC 7033 rule; this Actix test proves that missing
    /// `resource` is exposed as an Actix bad-request response.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[actix_web::test]
    async fn request_with_missing_resource() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let request = test::TestRequest::get()
            .uri(WELL_KNOWN_PATH)
            .insert_header(("host", "example.org"))
            .to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"missing resource parameter");
        Ok(())
    }

    /// Rejects requests where neither the URI nor `Host` header provides an authority.
    ///
    /// The request host is significant to WebFinger query routing.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4>.
    #[actix_web::test]
    async fn request_with_no_host() -> Result {
        let app = App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger));
        let app = test::init_service(app).await;
        let uri = format!("{WELL_KNOWN_PATH}?resource=acct%3Abad%40example.org");
        let request = test::TestRequest::get().uri(&uri).to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = to_bytes(response.into_body()).await?;
        assert_eq!(body.as_ref(), b"missing host");
        Ok(())
    }
}
