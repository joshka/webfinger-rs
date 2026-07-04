//! WebFinger lookup construction and result shaping.
//!
//! The viewer accepts either a WebFinger resource identifier such as `acct:alice@example.com` or a
//! full `/.well-known/webfinger` URL. This module is the table of contents for lookup behavior.
//! Request parsing, deployment policy, and result shaping live in focused child modules so future
//! changes to resource parsing, target policy, redirects, or body capture can be reviewed against
//! smaller tests.

mod policy;
mod request;
mod result;

use tracing::{error, info};

pub use policy::LookupPolicy;
pub use request::LookupRequest;
pub use result::{LookupError, LookupResult, LookupResultParts};

pub const MAX_BODY_BYTES: usize = 512 * 1024;
pub const ACCEPT_HEADER: &str = "application/jrd+json, application/json;q=0.9, */*;q=0.1";

/// Logs lookup failures at the protocol boundary.
///
/// The app module owns HTTP status mapping, but lookup owns the log message because it can add
/// protocol context without making route handling understand WebFinger internals.
pub fn log_lookup_error(error: &LookupError) {
    error!(?error, "webfinger lookup failed");
}

/// Logs target fetch outcomes.
///
/// Runtime entrypoints install their own tracing subscribers. Keep the fields low-cardinality
/// enough to filter by status and target host while still preserving the exact URL needed to
/// reproduce a WebFinger debugging failure.
pub fn log_lookup_result(
    request: &LookupRequest,
    status: u16,
    content_type: Option<&str>,
    truncated: bool,
) {
    info!(
        status,
        target_url = %request.target_url(),
        resource = %request.resource(),
        content_type = content_type.unwrap_or(""),
        truncated,
        "webfinger lookup result",
    );
}

/// Captured target body after enforcing the viewer's response-size limit.
pub struct CappedBody {
    /// Response body decoded lossily as UTF-8 for display.
    pub text: String,

    /// True when the target body exceeded `MAX_BODY_BYTES`.
    pub truncated: bool,
}

/// Applies the viewer body limit after a bounded read has completed.
///
/// The caller may pass at most `MAX_BODY_BYTES + 1` bytes. The extra byte records that the target
/// body was larger than the UI will render without preserving more attacker-controlled data.
pub fn cap_body_bytes(mut bytes: Vec<u8>) -> CappedBody {
    let truncated = bytes.len() > MAX_BODY_BYTES;
    if truncated {
        bytes.truncate(MAX_BODY_BYTES);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();

    CappedBody { text, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_request_uses_jrd_accept_header() {
        assert_eq!(
            ACCEPT_HEADER,
            "application/jrd+json, application/json;q=0.9, */*;q=0.1",
        );
    }

    #[test]
    fn caps_body_bytes_after_limit() {
        let body = cap_body_bytes(vec![b'a'; MAX_BODY_BYTES + 1]);

        assert!(body.truncated);
        assert_eq!(body.text.len(), MAX_BODY_BYTES);
    }

    #[test]
    fn exact_limit_is_not_truncated() {
        let body = cap_body_bytes(vec![b'a'; MAX_BODY_BYTES]);

        assert!(!body.truncated);
        assert_eq!(body.text.len(), MAX_BODY_BYTES);
    }
}
