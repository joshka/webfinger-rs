//! Deployment policy for target WebFinger fetches.
//!
//! Public deployments are same-origin by default so the viewer remains a debugging UI for the site
//! that serves it, not an unrestricted server-side fetch proxy. Local development gets an
//! off-origin exception so the local viewer can inspect arbitrary public resources and local
//! responders without relaxing production deployments.

use url::Url;

use super::LookupError;
use crate::config::LookupConfig;

/// Deployment-derived policy for deciding which target endpoints the viewer may fetch.
///
/// Public deployments are same-origin by default: the viewer may only fetch the WebFinger endpoint
/// for the host that served the UI. When the viewer itself is running on loopback under
/// loopback, off-origin targets are allowed so a local viewer can inspect public WebFinger
/// resources such as `acct:joshka@hachyderm.io` and local servers on another port. This keeps
/// production behavior conservative without requiring checked-in environment files for local
/// development.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupPolicy {
    viewer_origin: Origin,
    allow_off_origin_targets: bool,
    local_responder_port: u16,
}

impl LookupPolicy {
    /// Builds lookup policy from the request that served `/api/lookup`.
    ///
    /// This uses default lookup configuration. Runtime adapters that load configuration should call
    /// `from_viewer_url_and_config` instead.
    pub fn from_viewer_url(url: &Url) -> Self {
        Self::from_viewer_url_and_config(url, &LookupConfig::default())
    }

    /// Builds lookup policy from the request that served `/api/lookup` and runtime-neutral config.
    ///
    /// The request URL still decides whether the viewer is local or production-like: production
    /// hosts reject off-origin targets, while loopback hosts allow arbitrary targets for local
    /// testing. The config supplies local development details such as the default responder port for
    /// loopback `acct:` resources.
    pub fn from_viewer_url_and_config(url: &Url, config: &LookupConfig) -> Self {
        let viewer_origin = Origin::from_url(url);
        let allow_off_origin_targets = viewer_origin.host_is_loopback();
        Self {
            viewer_origin,
            allow_off_origin_targets,
            local_responder_port: config.local_responder_port,
        }
    }

    /// Enforces the target fetch policy before the runtime performs an outbound request.
    pub fn validate_target(&self, target_url: &Url) -> Result<(), LookupError> {
        let target_origin = Origin::from_url(target_url);
        if target_origin.same_origin(&self.viewer_origin) {
            return Ok(());
        }
        if self.allow_off_origin_targets {
            return Ok(());
        }

        Err(LookupError::OffOriginTarget {
            allowed_host: self.viewer_origin.host_for_message(),
        })
    }

    /// Returns true when the viewer request came from a loopback origin.
    ///
    /// This is intentionally derived from the request URL instead of a checked-in environment file.
    /// Local mode is more permissive because the developer is using a loopback-only viewer to debug
    /// arbitrary public resources and local responders.
    pub fn is_local_development(&self) -> bool {
        self.allow_off_origin_targets
    }

    /// Returns true when a host is one of the loopback spellings supported by local development.
    pub fn host_is_loopback(host: &str) -> bool {
        Origin::host_name_is_loopback(host)
    }

    /// Builds the default local responder origin for an inferred loopback resource host.
    ///
    /// `acct:` resources cannot carry a port. When the viewer is local and the resource host is
    /// loopback, derive an HTTP target on the configured local responder port so
    /// `acct:alice@localhost` can exercise the companion responder during development.
    pub fn local_responder_origin_for_host(&self, host: &str) -> Option<String> {
        if !self.is_local_development() || !Self::host_is_loopback(host) {
            return None;
        }

        let host = if host == "::1" {
            "[::1]".to_string()
        } else {
            host.to_string()
        };
        Some(format!("http://{host}:{}", self.local_responder_port))
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

    /// Returns true for loopback hostnames used by local development.
    ///
    /// The check intentionally covers the practical localhost spellings a developer can enter in
    /// the browser. It does not try to classify every private or special-purpose IP range because
    /// production deployments should stay same-origin unless the viewer itself is running locally.
    fn host_is_loopback(&self) -> bool {
        Self::host_name_is_loopback(&self.host)
    }

    /// Returns true for a normalized host name that refers to loopback.
    fn host_name_is_loopback(host: &str) -> bool {
        matches!(
            host.to_ascii_lowercase().as_str(),
            "localhost" | "127.0.0.1" | "::1"
        )
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

    fn local_policy_with_responder_port(port: u16) -> LookupPolicy {
        LookupPolicy::from_viewer_url_and_config(
            &Url::parse("http://127.0.0.1:8788/webfinger/api/lookup").unwrap(),
            &LookupConfig {
                local_responder_port: port,
            },
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
    fn allows_external_resource_from_local_viewer() {
        let request = LookupRequest::new(
            "acct:joshka@hachyderm.io".to_string(),
            Vec::new(),
            &local_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://hachyderm.io/.well-known/webfinger?resource=acct%3Ajoshka%40hachyderm.io",
        );
    }

    #[test]
    fn configured_local_responder_port_is_used_for_loopback_acct_resources() {
        let request = LookupRequest::new(
            "acct:alice@localhost".to_string(),
            Vec::new(),
            &local_policy_with_responder_port(8790),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "http://localhost:8790/.well-known/webfinger?resource=acct%3Aalice%40localhost",
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
            "this deployment only looks up WebFinger resources on joshka.net; run the viewer locally with a full localhost WebFinger URL for local server debugging",
        );
    }
}
