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

use crate::lookup::LookupResult;
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
/// Use this for viewer-level failures such as malformed resources or Worker fetch errors.
/// Non-htmx or cross-site callers are rejected by `server` before this rendering path.
pub fn lookup_error(message: &str) -> String {
    let state = StateView::bad("Failed");
    let template = LookupErrorTemplate { state, message };
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

    /// Full viewer-level error shown in the result body.
    message: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_error_keeps_header_short_and_body_specific() {
        let html = lookup_error("missing resource");

        assert!(html.contains(
            r#"<span id="state" class="state-text bad" hx-swap-oob="true">Failed</span>"#
        ));
        assert!(html.contains("<h2>Lookup Error</h2>"));
        assert!(html.contains("missing resource"));
    }
}
