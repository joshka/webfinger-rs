//! Target HTTP metadata rendered above the parsed JRD summary.
//!
//! These fields describe the WebFinger endpoint response, not the Worker request to `/api/lookup`.
//! Keeping transport facts separate from resource facts helps the template explain redirect,
//! status, and content-type behavior without mixing them into the JRD tables.

use crate::lookup::LookupResult;

/// Transport metadata for the target WebFinger response.
///
/// This intentionally describes the target endpoint, not the Worker request to `/api/lookup`.
/// Keeping it separate from the JRD summary prevents the template from mixing HTTP debugging facts
/// with resource/link facts.
pub struct MetaView {
    /// Target WebFinger HTTP status, not the Worker response status.
    pub status: String,

    /// Status color class derived from the target status.
    pub status_class: &'static str,

    /// Target `Content-Type`, or `(none)` when the header is absent.
    pub content_type: String,

    /// URL requested by the Worker.
    pub request_url: String,

    /// Redirect target reported by a manual `Location` header.
    pub redirect_location: String,

    /// True when the target response included a `Location` header.
    pub has_redirect_location: bool,
}

impl MetaView {
    /// Builds transport metadata shown above the JRD summary.
    ///
    /// Redirects are not followed by the Worker. If the target returns a `Location` header, the
    /// template shows it as the next endpoint to inspect without implying the Worker fetched it.
    pub fn from_result(result: &LookupResult) -> Self {
        let redirect_location = result.redirect_location.clone().unwrap_or_default();
        Self {
            status: result.status.to_string(),
            status_class: if (200..300).contains(&result.status) {
                "good"
            } else {
                "bad"
            },
            content_type: result
                .content_type
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
            request_url: result.request_url.clone(),
            has_redirect_location: !redirect_location.is_empty(),
            redirect_location,
        }
    }
}
