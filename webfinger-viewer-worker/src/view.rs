//! Server-rendered htmx fragments for lookup results.
//!
//! This module owns the render entry points and Askama template bindings. The heavier view-model
//! shaping lives in child modules: `state` for header state, `meta` for target HTTP metadata, and
//! `summary` for parsed JRD display rows. Keeping that split visible makes template changes
//! reviewable without reopening every JSON-display rule.
//!
//! The page is still served as one HTML response. That is intentional for a Worker commonly mounted
//! under a path such as `/webfinger`: separate asset routes would need their own path-prefix and
//! cache rules, while embedded assets make local development and deployment match.

mod meta;
mod state;
mod summary;

use askama::Template;

use crate::lookup::{LookupError, LookupResult};
use meta::MetaView;
use state::StateView;
use summary::{SummaryView, raw_body};

// htmx is pinned in package.json and vendored into assets/vendor to avoid both a production CDN
// dependency and a Cargo-time dependency on npm install. Check `npm view htmx.org version`, run
// `npm ci`, and copy `node_modules/htmx.org/dist/htmx.min.js` here when bumping the package.
const HTMX_JS: &str = include_str!("../assets/vendor/htmx.min.js");
const APP_CSS: &str = include_str!("../assets/app.css");
const APP_JS: &str = include_str!("../assets/app.js");

/// Renders the full viewer page shell.
///
/// CSS, htmx, and the small local behavior script are separate source files but are embedded into
/// the Worker response. That keeps deployment simple for a path-mounted Worker while avoiding a
/// monolithic HTML source file.
pub fn page() -> String {
    let page = PageTemplate {
        htmx_js: HTMX_JS,
        app_css: APP_CSS,
        app_js: APP_JS,
    };
    page.render().expect("page template renders")
}

/// Renders the htmx fragment swapped into the result region.
///
/// Target WebFinger HTTP status is displayed inside the fragment, not encoded as the Worker
/// response status. The Worker response is `200` for htmx so the browser swaps the result panel.
pub fn lookup_result(result: &LookupResult) -> String {
    let state = if result.truncated {
        StateView::warn("Fetched truncated body")
    } else {
        StateView::good("Fetched")
    };
    let template = LookupResultTemplate {
        state,
        meta: MetaView::from_result(result),
        summary: SummaryView::from_json(result.json.as_ref()),
        curl: result.curl.clone(),
        raw: raw_body(result),
    };
    template.render().expect("lookup result template renders")
}

/// Renders a failed lookup as a normal result fragment so htmx still swaps it into the page.
///
/// Use this for failures where no target HTTP response exists. The target-status slot stays visible
/// because this is a debugger: "not requested" and "no response" are more useful than hiding the
/// HTTP layer behind a generic error banner. Non-htmx or cross-site callers are rejected by
/// `server` before this rendering path.
pub fn lookup_error(error: &LookupError, target_url: Option<&str>) -> String {
    let state = StateView::bad("Failed");
    let diagnostic = ErrorDiagnosticView::from_lookup_error(error, target_url);
    let template = LookupErrorTemplate { state, diagnostic };
    template.render().expect("lookup error template renders")
}

#[derive(Template)]
#[template(path = "page.html")]
struct PageTemplate {
    /// Pinned htmx source embedded into the page.
    htmx_js: &'static str,

    /// Viewer stylesheet embedded into the page.
    app_css: &'static str,

    /// Small browser behavior script embedded into the page.
    app_js: &'static str,
}

#[derive(Template)]
#[template(path = "lookup_result.html")]
struct LookupResultTemplate<'a> {
    /// Header state swapped out-of-band by htmx.
    state: StateView<'a>,

    /// Target HTTP metadata displayed above parsed JRD content.
    meta: MetaView,

    /// Parsed JRD rows and link table data.
    summary: SummaryView,

    /// Copyable terminal reproduction command.
    curl: String,

    /// Pretty JSON or raw body rendered in the collapsed raw section.
    raw: String,
}

#[derive(Template)]
#[template(path = "lookup_error.html")]
struct LookupErrorTemplate<'a> {
    /// Header state swapped out-of-band by htmx.
    state: StateView<'a>,

    /// Developer-focused explanation of why no target HTTP status was available.
    diagnostic: ErrorDiagnosticView,
}

/// Diagnostic view for failures that happen before a target HTTP response exists.
///
/// Target `404`, `500`, and Cloudflare edge codes such as `522` are rendered by
/// `LookupResultTemplate` because the Worker received an HTTP response. This view is reserved for
/// viewer input, deployment policy, URL construction, and Worker transport failures where the most
/// useful debugging fact is why no target status code could be displayed.
struct ErrorDiagnosticView {
    /// Short phase label shown in the metadata strip.
    phase: &'static str,

    /// Target status slot text, such as `Not requested` or `No response`.
    target_status: &'static str,

    /// Endpoint text shown in the metadata strip.
    endpoint: String,

    /// Body heading for the diagnostic section.
    title: &'static str,

    /// Full error text copied from the concrete lookup error.
    message: String,

    /// Extra context that helps a developer decide the next check.
    help: &'static str,
}

impl ErrorDiagnosticView {
    /// Builds a diagnostic from a concrete lookup error and optional target URL.
    ///
    /// The optional target URL is only available after the viewer has parsed enough input to build
    /// a request. Keeping that distinction in the rendered output prevents "network" failures from
    /// looking the same as syntax or RFC resource-validation failures.
    fn from_lookup_error(error: &LookupError, target_url: Option<&str>) -> Self {
        match error {
            LookupError::Worker(_) => Self {
                phase: "Worker fetch",
                target_status: "No response",
                endpoint: target_url
                    .unwrap_or("Target request was not built")
                    .to_string(),
                title: "Fetch Error",
                message: error.to_string(),
                help: "The Worker attempted the endpoint but did not receive an HTTP response. Check local server availability, protocol, port, TLS, and Cloudflare Worker fetch restrictions.",
            },
            LookupError::OffOriginTarget { .. } => Self {
                phase: "Deployment policy",
                target_status: "Not requested",
                endpoint: "Blocked before fetch".to_string(),
                title: "Policy Error",
                message: error.to_string(),
                help: "This deployment is same-origin by default. Use the public site's own WebFinger resources here, or use local Wrangler with a full localhost WebFinger URL for local debugging.",
            },
            LookupError::Url(_)
            | LookupError::UnsupportedScheme(_)
            | LookupError::NotWebFingerUrl => Self {
                phase: "URL parsing",
                target_status: "Not requested",
                endpoint: "Invalid WebFinger endpoint".to_string(),
                title: "URL Error",
                message: error.to_string(),
                help: "Full endpoint input must be an absolute HTTP(S) URL whose path is exactly /.well-known/webfinger and whose query includes resource.",
            },
            LookupError::MissingResource
            | LookupError::CannotInferHost
            | LookupError::InvalidResource(_)
            | LookupError::ResourceTooLong { .. }
            | LookupError::TooManyRels { .. }
            | LookupError::RelTooLong { .. }
            | LookupError::TargetUrlTooLong { .. } => Self {
                phase: "Input validation",
                target_status: "Not requested",
                endpoint: "Not built".to_string(),
                title: "Input Error",
                message: error.to_string(),
                help: "Enter an absolute WebFinger resource, for example acct:alice@example.com, or paste the exact /.well-known/webfinger URL you want to inspect.",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_error_keeps_header_short_and_body_specific() {
        let html = lookup_error(&LookupError::MissingResource, None);

        assert!(html.contains(
            r#"<span id="state" class="state-text bad" hx-swap-oob="true">Failed</span>"#
        ));
        assert!(html.contains("<h2>Input Error</h2>"));
        assert!(html.contains("Input validation"));
        assert!(html.contains("Target status"));
        assert!(html.contains("Not built"));
        assert!(html.contains("missing resource"));
    }

    #[test]
    fn invalid_resource_error_keeps_parser_reason_visible() {
        let error = LookupError::InvalidResource(webfinger_rs::ResourceError::RelativeReference);
        let html = lookup_error(&error, None);

        assert!(html.contains("resource must be an absolute URI such as"));
        assert!(html.contains("validation error: resource must be an absolute URI"));
    }

    #[test]
    fn lookup_error_shows_target_url_for_worker_failures() {
        let error =
            LookupError::Worker(worker::Error::RustError("Network connection lost.".into()));
        let html = lookup_error(
            &error,
            Some("http://localhost:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost"),
        );

        assert!(html.contains("Worker fetch"));
        assert!(html.contains("No response"));
        assert!(html.contains("http://localhost:8787/.well-known/webfinger"));
        assert!(html.contains("Network connection lost."));
    }
}
