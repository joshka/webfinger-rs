//! Deployment policy for target WebFinger fetches.
//!
//! Public deployments are same-origin by default so the viewer remains a debugging UI for the site
//! that serves it, not an unrestricted server-side fetch proxy. Local Wrangler development gets a
//! narrow loopback exception so a viewer on one localhost port can inspect a responder on another.

use url::Url;

use super::LookupError;

/// Deployment-derived policy for deciding which target endpoints the viewer may fetch.
///
/// Public deployments are same-origin by default: the viewer may only fetch the WebFinger endpoint
/// for the host that served the UI. When the viewer itself is running on loopback under
/// `wrangler dev`, loopback target URLs are also allowed so a local viewer on port `8788` can
/// inspect a local WebFinger server on another port such as `8787`. This keeps production behavior
/// conservative without requiring checked-in environment files for local development.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupPolicy {
    viewer_origin: Origin,
    allow_loopback_targets: bool,
}

impl LookupPolicy {
    /// Builds lookup policy from the request that served `/api/lookup`.
    ///
    /// The request URL is the only configuration source on purpose. This Worker is meant to be a
    /// reusable tool that can be deployed under different hostnames without editing repo-local env
    /// files. Validate the behavior by changing only the request host: production-like hosts reject
    /// off-origin targets, while loopback hosts allow loopback WebFinger URLs for local testing.
    pub fn from_viewer_url(url: &Url) -> Self {
        let viewer_origin = Origin::from_url(url);
        let allow_loopback_targets = viewer_origin.host_is_loopback();
        Self {
            viewer_origin,
            allow_loopback_targets,
        }
    }

    /// Enforces the target fetch policy before the Worker performs an outbound request.
    pub fn validate_target(&self, target_url: &Url) -> Result<(), LookupError> {
        let target_origin = Origin::from_url(target_url);
        if target_origin.same_origin(&self.viewer_origin) {
            return Ok(());
        }
        if self.allow_loopback_targets && target_origin.host_is_loopback() {
            return Ok(());
        }

        Err(LookupError::OffOriginTarget {
            allowed_host: self.viewer_origin.host_for_message(),
        })
    }
}

/// Minimal origin identity used by the lookup policy.
///
/// `Url` has more detail than the policy needs. Keeping only scheme, host, and effective port makes
/// the same-origin decision explicit and testable, including default ports such as `https:443`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Origin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl Origin {
    /// Extracts the origin fields needed for same-origin comparison.
    fn from_url(url: &Url) -> Self {
        Self {
            scheme: url.scheme().to_string(),
            host: url.host_str().unwrap_or_default().to_ascii_lowercase(),
            port: url.port_or_known_default(),
        }
    }

    /// Returns true when two URLs have the same scheme, host, and effective port.
    fn same_origin(&self, other: &Self) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }

    /// Returns true for loopback hostnames used by local Wrangler development.
    ///
    /// The check intentionally covers the practical localhost spellings a developer can enter in
    /// the browser. It does not try to classify every private or special-purpose IP range because
    /// production deployments should stay same-origin unless the viewer itself is running locally.
    fn host_is_loopback(&self) -> bool {
        matches!(self.host.as_str(), "localhost" | "127.0.0.1" | "::1")
    }

    /// Formats the allowed host for user-facing policy errors.
    fn host_for_message(&self) -> String {
        if let Some(port) = self.non_default_port() {
            format!("{}:{port}", self.host)
        } else {
            self.host.clone()
        }
    }

    /// Returns the port only when it should be shown in a user-facing host.
    ///
    /// The viewer reports a host for policy errors, not a full origin. Hiding `80` and `443` keeps
    /// the message readable even when the URL parser filled in a known default port.
    fn non_default_port(&self) -> Option<u16> {
        match self.port {
            Some(80 | 443) => None,
            port => port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lookup::LookupRequest;

    fn production_policy() -> LookupPolicy {
        LookupPolicy::from_viewer_url(
            &Url::parse("https://example.com/webfinger/api/lookup").unwrap(),
        )
    }

    fn local_policy() -> LookupPolicy {
        LookupPolicy::from_viewer_url(
            &Url::parse("http://127.0.0.1:8788/webfinger/api/lookup").unwrap(),
        )
    }

    #[test]
    fn rejects_off_origin_resource_on_public_host() {
        let policy = LookupPolicy::from_viewer_url(
            &Url::parse("https://example.com/webfinger/api/lookup").unwrap(),
        );

        let error = LookupRequest::new("acct:alice@other.example".to_string(), Vec::new(), &policy)
            .unwrap_err();

        assert!(matches!(error, LookupError::OffOriginTarget { .. }));
    }

    #[test]
    fn allows_same_origin_resource_on_public_host() {
        let policy = LookupPolicy::from_viewer_url(
            &Url::parse("https://example.com/webfinger/api/lookup").unwrap(),
        );
        let request =
            LookupRequest::new("acct:alice@example.com".to_string(), Vec::new(), &policy).unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com",
        );
    }

    #[test]
    fn allows_loopback_webfinger_url_from_local_viewer() {
        let request = LookupRequest::new(
            "http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost"
                .to_string(),
            Vec::new(),
            &local_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost",
        );
    }

    #[test]
    fn rejects_loopback_webfinger_url_from_public_viewer() {
        let error = LookupRequest::new(
            "http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost"
                .to_string(),
            Vec::new(),
            &production_policy(),
        )
        .unwrap_err();

        assert!(matches!(error, LookupError::OffOriginTarget { .. }));
    }

    #[test]
    fn omits_default_port_from_policy_error_host() {
        let policy =
            LookupPolicy::from_viewer_url(&Url::parse("https://joshka.net/webfinger").unwrap());
        let error = LookupRequest::new("acct:alice@example.com".to_string(), Vec::new(), &policy)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "this deployment only looks up WebFinger resources on joshka.net; use local Wrangler with a full localhost WebFinger URL for local server debugging",
        );
    }
}
