//! Viewer request parsing and target URL construction.
//!
//! This module is the boundary where raw form query parameters become a bounded, policy-checked
//! WebFinger request. Keep protocol validation here so the Worker fetch path can assume the target
//! URL has already passed the deployment policy.

use url::Url;
use webfinger_rs::{Resource, WELL_KNOWN_PATH};

use super::{LookupError, LookupPolicy};

// These limits bound what the viewer accepts and re-renders; they are not WebFinger protocol
// limits. They are deliberately character-based because the UI displays these values as text, and
// the final URL cap catches percent-encoding growth before the Worker performs the outbound fetch.
const MAX_RESOURCE_CHARS: usize = 2_048;
const MAX_REL_CHARS: usize = 512;
const MAX_RELS: usize = 16;
const MAX_TARGET_URL_CHARS: usize = 4_096;

/// Parsed lookup request from the browser API.
///
/// The stored `target_url` is the actual URL fetched by Cloudflare. Keeping it next to the original
/// `resource` and selected `rels` lets the UI show both the user's input and the normalized
/// endpoint without recalculating protocol details in JavaScript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupRequest {
    /// Resource string supplied by the user or extracted from a full WebFinger URL.
    resource: String,

    /// Relation filters that should be sent as repeated `rel` query parameters.
    rels: Vec<String>,

    /// Absolute `/.well-known/webfinger` endpoint fetched by the Worker runtime.
    target_url: Url,
}

impl LookupRequest {
    /// Builds a lookup request from the viewer API query string.
    ///
    /// Empty `rel` query parameters are ignored because the browser UI may send optional text-box
    /// state. Unknown query parameters are ignored so deployment platforms can add their own
    /// routing metadata without breaking lookups. Resource, relation, and target URL sizes are
    /// bounded before any outbound fetch so the Worker cannot be used to render or request
    /// unbounded user input.
    pub fn from_url_query(url: &Url, policy: &LookupPolicy) -> Result<Self, LookupError> {
        let mut resource = None;
        let mut rels = Vec::new();
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "resource" => resource = Some(value.into_owned()),
                "rel" => {
                    for rel in value.split([',', '\n']).map(str::trim) {
                        if !rel.is_empty() {
                            rels.push(rel.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        let resource = resource.ok_or(LookupError::MissingResource)?;
        Self::new(resource, rels, policy)
    }

    /// Builds a lookup request from validated viewer input.
    ///
    /// Full WebFinger URLs preserve their original endpoint unless the caller supplies new `rel`
    /// filters. Resource identifiers normally derive an HTTPS endpoint from the resource host,
    /// matching RFC 7033 discovery expectations. Local Wrangler development is the exception:
    /// loopback full URLs are normalized to HTTP, and loopback resource identifiers derive
    /// `http://<host>:8787` so they can reach a companion local responder Worker.
    pub fn new(
        resource: String,
        rels: Vec<String>,
        policy: &LookupPolicy,
    ) -> Result<Self, LookupError> {
        validate_resource(&resource)?;
        validate_rels(&rels)?;

        let target_url = if points_at_webfinger_endpoint(&resource) {
            webfinger_url(&resource, &rels, policy)?
        } else {
            let _validated = resource.parse::<Resource>()?;
            resource_url(&resource, &rels, policy)?
        };
        validate_target_url(&target_url)?;
        policy.validate_target(&target_url)?;

        Ok(Self {
            resource,
            rels,
            target_url,
        })
    }

    /// Returns the user-facing resource string being queried.
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Returns relation filters that were sent to the target endpoint.
    pub fn rels(&self) -> &[String] {
        &self.rels
    }

    /// Returns the policy-checked endpoint URL fetched by the Worker.
    pub fn target_url(&self) -> &Url {
        &self.target_url
    }
}

/// Returns true when the user supplied the WebFinger endpoint itself.
///
/// This is a permissive classifier, not full validation. Full URL validation happens in
/// `webfinger_url`, which lets the viewer distinguish "treat this as a full endpoint" from
/// "accept this endpoint as fetchable" and produce more specific errors.
fn points_at_webfinger_endpoint(input: &str) -> bool {
    let Ok(url) = Url::parse(input) else {
        return false;
    };
    url.path() == WELL_KNOWN_PATH
}

/// Normalizes a full WebFinger URL supplied by the user.
///
/// A full URL is useful when debugging an exact endpoint or reproducing another client's request.
/// If the viewer supplies `rel` filters, they replace the URL's existing `rel` parameters so the UI
/// has one obvious source of truth for active filters.
fn webfinger_url(input: &str, rels: &[String], policy: &LookupPolicy) -> Result<Url, LookupError> {
    let mut url = Url::parse(input)?;
    if url.path() != WELL_KNOWN_PATH {
        return Err(LookupError::NotWebFingerUrl);
    }
    if !matches!(url.scheme(), "https" | "http") {
        return Err(LookupError::UnsupportedScheme(url.scheme().to_string()));
    }
    normalize_local_loopback_scheme(&mut url, policy)?;

    let resource = url
        .query_pairs()
        .find_map(|(key, value)| (key == "resource").then(|| value.into_owned()))
        .ok_or(LookupError::MissingResource)?;
    validate_resource(&resource)?;
    let _validated = resource.parse::<Resource>()?;

    if !rels.is_empty() {
        url.set_query(None);
        let mut query = url.query_pairs_mut();
        query.append_pair("resource", &resource);
        for rel in rels {
            query.append_pair("rel", rel);
        }
    }

    Ok(url)
}

/// Builds the standard WebFinger endpoint URL for a resource identifier.
///
/// The viewer derives HTTPS endpoints for normal resource input. In local development, loopback
/// hosts use the repo's default local responder origin so `acct:alice@localhost` reaches
/// `http://localhost:8787/.well-known/webfinger`. Use the full WebFinger URL input path when
/// debugging a different local port or an exact query string from another client.
fn resource_url(
    resource: &str,
    rels: &[String],
    policy: &LookupPolicy,
) -> Result<Url, LookupError> {
    let host = resource_host(resource)?;
    let origin = policy
        .local_responder_origin_for_host(&host)
        .unwrap_or_else(|| format!("https://{host}"));
    let mut url = Url::parse(&format!("{origin}{WELL_KNOWN_PATH}"))?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("resource", resource);
        for rel in rels {
            query.append_pair("rel", rel);
        }
    }
    Ok(url)
}

/// Normalizes loopback HTTPS URLs to HTTP during local Wrangler development.
///
/// Wrangler dev servers listen on plain HTTP. This viewer accepts `https://localhost:8787/...`
/// because it is the standard WebFinger shape many people type first, but local mode rewrites that
/// target to `http://localhost:8787/...` before the Worker fetches it. Production-like deployments
/// never rewrite schemes.
fn normalize_local_loopback_scheme(
    url: &mut Url,
    policy: &LookupPolicy,
) -> Result<(), LookupError> {
    if !policy.is_local_development() || url.scheme() != "https" {
        return Ok(());
    }
    let Some(host) = url.host_str() else {
        return Ok(());
    };
    if LookupPolicy::host_is_loopback(host) {
        url.set_scheme("http")
            .map_err(|_| LookupError::UnsupportedScheme(url.scheme().to_string()))?;
    }
    Ok(())
}

/// Validates the user-visible resource before target URL construction.
///
/// The limit is intentionally generous enough for ordinary `acct:` identifiers and URI resources,
/// including local development URLs, but small enough to keep rendered error/result fragments and
/// derived target URLs bounded.
fn validate_resource(resource: &str) -> Result<(), LookupError> {
    if resource.chars().count() > MAX_RESOURCE_CHARS {
        return Err(LookupError::ResourceTooLong {
            max: MAX_RESOURCE_CHARS,
        });
    }
    Ok(())
}

/// Validates relation filters supplied by the form.
///
/// Relation filters are repeated into the target query string and shown back in the UI, so both
/// count and per-value length are capped. These limits apply to rel values submitted through the
/// viewer controls. A full WebFinger URL can still preserve unusual existing query strings for
/// debugging; the final target URL cap is the guard for that exact-reproduction path.
fn validate_rels(rels: &[String]) -> Result<(), LookupError> {
    if rels.len() > MAX_RELS {
        return Err(LookupError::TooManyRels { max: MAX_RELS });
    }
    for rel in rels {
        if rel.chars().count() > MAX_REL_CHARS {
            return Err(LookupError::RelTooLong { max: MAX_REL_CHARS });
        }
    }
    Ok(())
}

/// Validates the final URL sent through the Worker runtime.
///
/// This is the last guard after percent-encoding, relation expansion, and full-URL preservation.
/// It keeps curl rendering, logs, and the outbound request line within a predictable debugging-tool
/// size without blocking localhost or private-host experiments during `wrangler dev`.
fn validate_target_url(url: &Url) -> Result<(), LookupError> {
    if url.as_str().chars().count() > MAX_TARGET_URL_CHARS {
        return Err(LookupError::TargetUrlTooLong {
            max: MAX_TARGET_URL_CHARS,
        });
    }
    Ok(())
}

/// Infers the host that owns a WebFinger resource.
///
/// `acct:` resources use the domain after the final `@`. URI resources use their URL host. Other
/// resource schemes may still be valid WebFinger identifiers, but this viewer cannot infer where to
/// query them without a host, so callers should provide a full WebFinger URL for those cases.
fn resource_host(resource: &str) -> Result<String, LookupError> {
    if let Some(account) = resource.strip_prefix("acct:") {
        let host = account
            .rsplit_once('@')
            .map(|(_, host)| host)
            .filter(|host| !host.is_empty())
            .ok_or(LookupError::CannotInferHost)?;
        return Ok(host.to_string());
    }

    let url = Url::parse(resource).map_err(|_| LookupError::CannotInferHost)?;
    let host = url.host_str().ok_or(LookupError::CannotInferHost)?;
    Ok(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_policy() -> LookupPolicy {
        LookupPolicy::from_viewer_url(
            &Url::parse("https://example.com/webfinger/api/lookup").unwrap(),
        )
    }

    fn local_policy() -> LookupPolicy {
        LookupPolicy::from_viewer_url(
            &Url::parse("http://localhost:8790/webfinger/api/lookup").unwrap(),
        )
    }

    #[test]
    fn builds_acct_target_url() {
        let request = LookupRequest::new(
            "acct:alice@example.com".to_string(),
            vec!["http://webfinger.net/rel/profile-page".to_string()],
            &production_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com&rel=http%3A%2F%2Fwebfinger.net%2Frel%2Fprofile-page",
        );
    }

    #[test]
    fn builds_uri_resource_target_url() {
        let request = LookupRequest::new(
            "https://example.com/users/alice".to_string(),
            Vec::new(),
            &production_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://example.com/.well-known/webfinger?resource=https%3A%2F%2Fexample.com%2Fusers%2Falice",
        );
    }

    #[test]
    fn accepts_full_webfinger_url() {
        let request = LookupRequest::new(
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com"
                .to_string(),
            Vec::new(),
            &production_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com",
        );
    }

    #[test]
    fn local_viewer_derives_loopback_acct_resource_to_default_responder_port() {
        let request = LookupRequest::new(
            "acct:alice@localhost".to_string(),
            Vec::new(),
            &local_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "http://localhost:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost",
        );
    }

    #[test]
    fn local_viewer_derives_ipv4_loopback_acct_resource_to_default_responder_port() {
        let request = LookupRequest::new(
            "acct:alice@127.0.0.1".to_string(),
            Vec::new(),
            &local_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40127.0.0.1",
        );
    }

    #[test]
    fn local_viewer_normalizes_https_loopback_webfinger_url_to_http() {
        let request = LookupRequest::new(
            "https://localhost:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost"
                .to_string(),
            Vec::new(),
            &local_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "http://localhost:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost",
        );
    }

    #[test]
    fn replaces_full_url_relation_filters_when_requested() {
        let request = LookupRequest::new(
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com&rel=self"
                .to_string(),
            vec!["http://webfinger.net/rel/profile-page".to_string()],
            &production_policy(),
        )
        .unwrap();

        assert_eq!(
            request.target_url().as_str(),
            "https://example.com/.well-known/webfinger?resource=acct%3Aalice%40example.com&rel=http%3A%2F%2Fwebfinger.net%2Frel%2Fprofile-page",
        );
    }

    #[test]
    fn rejects_relative_resources() {
        let error =
            LookupRequest::new("alice".to_string(), Vec::new(), &production_policy()).unwrap_err();

        assert!(matches!(error, LookupError::WebFinger(_)));
    }

    #[test]
    fn rejects_full_webfinger_urls_without_resource() {
        let error = LookupRequest::new(
            "https://example.com/.well-known/webfinger?rel=self".to_string(),
            Vec::new(),
            &production_policy(),
        )
        .unwrap_err();

        assert!(matches!(error, LookupError::MissingResource));
    }

    #[test]
    fn rejects_overlong_resources() {
        let resource = format!("acct:{}@example.com", "a".repeat(MAX_RESOURCE_CHARS));
        let error = LookupRequest::new(resource, Vec::new(), &production_policy()).unwrap_err();

        assert!(matches!(error, LookupError::ResourceTooLong { .. }));
    }

    #[test]
    fn rejects_too_many_relation_filters() {
        let rels = (0..=MAX_RELS)
            .map(|index| format!("https://example.com/rel/{index}"))
            .collect();
        let error = LookupRequest::new(
            "acct:alice@example.com".to_string(),
            rels,
            &production_policy(),
        )
        .unwrap_err();

        assert!(matches!(error, LookupError::TooManyRels { .. }));
    }

    #[test]
    fn rejects_overlong_relation_filters() {
        let rel = format!("https://example.com/rel/{}", "a".repeat(MAX_REL_CHARS));
        let error = LookupRequest::new(
            "acct:alice@example.com".to_string(),
            vec![rel],
            &production_policy(),
        )
        .unwrap_err();

        assert!(matches!(error, LookupError::RelTooLong { .. }));
    }

    #[test]
    fn rejects_overlong_target_urls() {
        let rels = (0..MAX_RELS)
            .map(|index| {
                format!(
                    "https://example.com/{index}/{}",
                    "a".repeat(MAX_REL_CHARS - 24)
                )
            })
            .collect();
        let error = LookupRequest::new(
            "acct:alice@example.com".to_string(),
            rels,
            &production_policy(),
        )
        .unwrap_err();

        assert!(matches!(error, LookupError::TargetUrlTooLong { .. }));
    }
}
