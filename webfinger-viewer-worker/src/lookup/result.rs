//! Lookup result and error types rendered by the viewer.
//!
//! The fetch adapter returns transport metadata and a capped body, while this module shapes those
//! pieces into the browser-facing result. Keeping curl construction and JSON parsing here keeps the
//! outbound fetch path focused on side effects.

use serde::Serialize;
use thiserror::Error;

/// Browser-facing result returned by `/api/lookup`.
///
/// The UI renders structured fields from `json` when the body parses as JSON, but it also keeps the
/// raw `body` so malformed or non-JRD responses remain inspectable.
#[derive(Debug, Serialize)]
pub struct LookupResult {
    /// URL the Worker attempted to fetch.
    pub request_url: String,

    /// Redirect target from the target response `Location` header, if present.
    pub redirect_location: Option<String>,

    /// Resource being queried.
    pub resource: String,

    /// Relation filters sent to the endpoint.
    pub rels: Vec<String>,

    /// Reproduction command for a terminal.
    pub curl: String,

    /// HTTP status returned by the target endpoint.
    pub status: u16,

    /// Target endpoint `Content-Type`, if present.
    pub content_type: Option<String>,

    /// Response body decoded lossily as UTF-8 for display.
    pub body: String,

    /// Parsed JSON body when the response is valid JSON.
    pub json: Option<serde_json::Value>,

    /// True when `body` was capped at `MAX_BODY_BYTES`.
    pub truncated: bool,
}

/// Captured target response pieces needed to build a browser-facing lookup result.
///
/// The Worker fetch path collects these values from separate APIs: the target URL from request
/// parsing, selected headers from the target response, and a capped body reader. Grouping them here
/// keeps `LookupResult::new` readable at call sites and makes it clear which raw protocol facts feed
/// derived display fields such as `curl` and `json`.
pub struct LookupResultParts {
    /// URL the Worker attempted to fetch.
    pub request_url: String,

    /// Redirect target from the target response `Location` header, if present.
    pub redirect_location: Option<String>,

    /// Resource being queried.
    pub resource: String,

    /// Relation filters sent to the endpoint.
    pub rels: Vec<String>,

    /// HTTP status returned by the target endpoint.
    pub status: u16,

    /// Target endpoint `Content-Type`, if present.
    pub content_type: Option<String>,

    /// Response body decoded lossily as UTF-8 for display.
    pub body: String,

    /// True when `body` was capped at `MAX_BODY_BYTES`.
    pub truncated: bool,
}

impl LookupResult {
    /// Builds a browser-facing result from the captured target response.
    ///
    /// JSON parsing is intentionally best-effort. A malformed JRD or HTML error page is still a
    /// useful debugging result, so parse failure becomes `json: None` while the raw body remains
    /// copyable in the UI.
    pub fn new(parts: LookupResultParts) -> Self {
        let json = serde_json::from_str::<serde_json::Value>(&parts.body).ok();
        let curl = curl_command(&parts.request_url);

        let LookupResultParts {
            request_url,
            redirect_location,
            resource,
            rels,
            status,
            content_type,
            body,
            truncated,
        } = parts;

        Self {
            request_url,
            redirect_location,
            resource,
            rels,
            curl,
            status,
            content_type,
            body,
            json,
            truncated,
        }
    }
}

/// Errors that can occur while preparing or running a lookup.
#[derive(Debug, Error)]
pub enum LookupError {
    /// The viewer API did not include a `resource` query parameter.
    #[error("missing resource")]
    MissingResource,

    /// The resource is valid enough to enter the viewer but not enough to infer a target host.
    #[error("resource must be an absolute URI or a full WebFinger URL")]
    CannotInferHost,

    /// A full URL was supplied, but its path is not the standard WebFinger endpoint.
    #[error("full WebFinger URLs must use /.well-known/webfinger")]
    NotWebFingerUrl,

    /// Full WebFinger URLs are limited to ordinary HTTP(S) endpoints.
    #[error("unsupported URL scheme `{0}`")]
    UnsupportedScheme(String),

    /// The resource input is too large for the viewer to safely render and fetch.
    #[error("resource is too long; maximum is {max} characters")]
    ResourceTooLong { max: usize },

    /// Too many relation filters were supplied.
    #[error("too many relation filters; maximum is {max}")]
    TooManyRels { max: usize },

    /// One relation filter is too large for the viewer to safely render and fetch.
    #[error("relation filter is too long; maximum is {max} characters")]
    RelTooLong { max: usize },

    /// The final target URL is too large after encoding and relation expansion.
    #[error("target WebFinger URL is too long; maximum is {max} characters")]
    TargetUrlTooLong { max: usize },

    /// The deployment only permits same-origin lookups for public traffic.
    #[error(
        "this deployment only looks up WebFinger resources on {allowed_host}; use local Wrangler with a full localhost WebFinger URL for local server debugging"
    )]
    OffOriginTarget { allowed_host: String },

    /// URL parsing failed while building the target endpoint.
    #[error(transparent)]
    Url(#[from] url::ParseError),

    /// The resource string failed `webfinger-rs` resource validation.
    #[error(
        "resource must be an absolute URI such as `acct:alice@example.com`, or a full `https://example.com/.well-known/webfinger?resource=...` URL; validation error: {0}"
    )]
    InvalidResource(#[source] webfinger_rs::ResourceError),

    /// Cloudflare Worker request, response, or header handling failed.
    #[error(transparent)]
    Worker(#[from] worker::Error),
}

/// Builds the curl command shown by the UI.
///
/// This command mirrors the Worker's target URL and `Accept` preference closely enough for manual
/// reproduction. It intentionally does not include Worker-only details such as htmx headers.
fn curl_command(url: &str) -> String {
    format!(
        "curl -i -H 'Accept: application/jrd+json' '{}'",
        shell_single_quote(url)
    )
}

/// Quotes a string for a POSIX shell single-quoted argument.
///
/// WebFinger URLs can contain user-controlled query values, so the displayed curl command must be
/// copyable without letting a quote in the URL break out of the shell argument.
fn shell_single_quote(input: &str) -> String {
    input.replace('\'', "'\"'\"'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_quotes_single_quotes_in_target_url() {
        let result = LookupResult::new(LookupResultParts {
            request_url:
                "https://example.com/.well-known/webfinger?resource=acct%3Aalice%27test%40example.com"
                    .to_string(),
            redirect_location: None,
            resource: "acct:alice'test@example.com".to_string(),
            rels: Vec::new(),
            status: 200,
            content_type: None,
            body: "{}".to_string(),
            truncated: false,
        });

        assert_eq!(
            result.curl,
            "curl -i -H 'Accept: application/jrd+json' 'https://example.com/.well-known/webfinger?resource=acct%3Aalice%27test%40example.com'",
        );
    }

    #[test]
    fn parses_json_body_when_valid() {
        let result = LookupResult::new(LookupResultParts {
            request_url:
                "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com"
                    .to_string(),
            redirect_location: None,
            resource: "acct:alice@example.com".to_string(),
            rels: Vec::new(),
            status: 200,
            content_type: Some("application/jrd+json".to_string()),
            body: r#"{"subject":"acct:alice@example.com"}"#.to_string(),
            truncated: false,
        });

        assert!(result.json.is_some());
    }

    #[test]
    fn keeps_malformed_body_inspectable() {
        let result = LookupResult::new(LookupResultParts {
            request_url:
                "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com"
                    .to_string(),
            redirect_location: None,
            resource: "acct:alice@example.com".to_string(),
            rels: Vec::new(),
            status: 500,
            content_type: Some("text/html".to_string()),
            body: "<h1>error</h1>".to_string(),
            truncated: false,
        });

        assert!(result.json.is_none());
        assert_eq!(result.body, "<h1>error</h1>");
    }
}
