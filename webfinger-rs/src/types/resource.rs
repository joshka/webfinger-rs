use std::fmt;
use std::str::FromStr;

use http::Uri;

/// Errors that can occur while parsing a WebFinger resource URI.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    /// The resource is a relative reference instead of an absolute URI.
    #[error("resource must be an absolute URI")]
    RelativeReference,

    /// The resource contains raw text outside the URI character set.
    ///
    /// Resource URI text must be ASCII and every byte must be allowed by RFC 3986 as an
    /// `unreserved`, `reserved`, or percent-escape marker byte. Characters outside that set, such
    /// as `{`, `|`, `^`, and non-ASCII code points, must be percent-encoded before parsing.
    #[error("resource contains invalid URI characters")]
    InvalidCharacters,

    /// The resource contains a malformed percent escape.
    #[error("resource contains invalid percent encoding")]
    InvalidPercentEncoding,

    /// The resource is an invalid HTTP or HTTPS URI.
    #[error(transparent)]
    InvalidHttpUri(#[from] http::uri::InvalidUri),

    /// The resource is an HTTP or HTTPS URI without an authority.
    #[error("HTTP and HTTPS resources must include an authority")]
    MissingHttpAuthority,
}

/// A WebFinger resource URI.
///
/// RFC 7033 uses the `resource` query parameter for the query target, which is a URI rather than a
/// relative reference. `Resource` stores that URI text after checking the URI syntax that this crate
/// relies on at request boundaries.
///
/// Validation is intentionally conservative:
///
/// - the value must start with an RFC 3986 URI scheme;
/// - the value must contain only raw RFC 3986 URI characters;
/// - every `%` must start a complete percent escape;
/// - raw non-ASCII text must already be percent-encoded; and
/// - `http` and `https` resources must use the `//authority` form before their host is exposed
///   through [`Resource::host`].
///
/// Common valid resources include `acct:carol@example.com` and
/// `https://example.org/users/carol`.
///
/// # Examples
///
/// Parse a valid `acct:` resource:
///
/// ```rust
/// use webfinger_rs::Resource;
///
/// let resource = "acct:carol@example.com".parse::<Resource>()?;
/// assert_eq!(resource.as_str(), "acct:carol@example.com");
/// # Ok::<(), webfinger_rs::ResourceError>(())
/// ```
///
/// Raw characters outside the URI character set are rejected. Percent-encode them inside the
/// resource URI before putting that URI in the outer WebFinger query string:
///
/// ```rust
/// use webfinger_rs::{Resource, ResourceError};
///
/// let error = "acct:carol{admin}@example.com"
///     .parse::<Resource>()
///     .unwrap_err();
/// assert!(matches!(error, ResourceError::InvalidCharacters));
///
/// let resource = "acct:carol%7Badmin%7D@example.com".parse::<Resource>()?;
/// assert_eq!(resource.as_str(), "acct:carol%7Badmin%7D@example.com");
/// # Ok::<(), webfinger_rs::ResourceError>(())
/// ```
///
/// HTTP(S) resources must include an authority so host inference cannot treat opaque URI text as a
/// host:
///
/// ```rust
/// use webfinger_rs::{Resource, ResourceError};
///
/// let error = "https:example.org/profile"
///     .parse::<Resource>()
///     .unwrap_err();
/// assert!(matches!(error, ResourceError::MissingHttpAuthority));
///
/// let resource = "https://example.org/profile".parse::<Resource>()?;
/// assert_eq!(resource.host(), Some("example.org"));
/// # Ok::<(), webfinger_rs::ResourceError>(())
/// ```
///
/// See [RFC 7033 section 4.1] for the `resource` parameter, [RFC 3986 section 2.1] for percent
/// encoding, [RFC 3986 section 2.2] for reserved characters, [RFC 3986 section 2.3] for
/// unreserved characters, [RFC 3986 section 3.1] for URI schemes, and [RFC 3986 section 3.2] for
/// authority.
///
/// [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
/// [RFC 3986 section 2.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1
/// [RFC 3986 section 2.2]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.2
/// [RFC 3986 section 2.3]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.3
/// [RFC 3986 section 3.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-3.1
/// [RFC 3986 section 3.2]: https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Resource {
    text: String,
    host: Option<String>,
}

impl Resource {
    /// Returns the resource URI as a string slice.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns the resource as an [`http::Uri`] when it fits that representation.
    ///
    /// WebFinger resources can use schemes such as `acct:` that are valid URI strings but do not
    /// expose a host through [`http::Uri`]. This accessor is mainly useful for hierarchical
    /// resources such as `https://example.org/users/carol`.
    pub fn uri(&self) -> Option<Uri> {
        Uri::try_from(self.as_str()).ok()
    }

    /// Returns the host from the resource's [`http::Uri`] representation, when present.
    ///
    /// URI schemes such as `acct:` do not have a host in [`http::Uri`], so this returns `None` for
    /// those resources.
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl AsRef<str> for Resource {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for Resource {
    type Err = ResourceError;

    fn from_str(resource: &str) -> Result<Self, Self::Err> {
        let host = validate_resource(resource)?;
        Ok(Self {
            text: resource.to_string(),
            host,
        })
    }
}

impl TryFrom<String> for Resource {
    type Error = ResourceError;

    fn try_from(resource: String) -> Result<Self, Self::Error> {
        let host = validate_resource(&resource)?;
        Ok(Self {
            text: resource,
            host,
        })
    }
}

impl TryFrom<&str> for Resource {
    type Error = ResourceError;

    fn try_from(resource: &str) -> Result<Self, Self::Error> {
        resource.parse()
    }
}

fn validate_resource(resource: &str) -> Result<Option<String>, ResourceError> {
    let Some(scheme) = scheme(resource) else {
        return Err(ResourceError::RelativeReference);
    };
    if !resource.is_ascii() {
        return Err(ResourceError::InvalidCharacters);
    }
    validate_uri_characters(resource)?;
    validate_percent_escapes(resource)?;
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        // WebFinger only needs host inference for hierarchical HTTP(S) resources. RFC 3986
        // section 3.2 attaches an authority to URIs that begin their hier-part with `//`; opaque
        // forms like `http:foo` must not produce a synthetic host.
        if !resource[scheme.len()..].starts_with("://") {
            return Err(ResourceError::MissingHttpAuthority);
        }
        let uri = Uri::try_from(resource).map_err(ResourceError::InvalidHttpUri)?;
        let Some(host) = uri.host() else {
            return Err(ResourceError::MissingHttpAuthority);
        };
        return Ok(Some(host.to_string()));
    }
    Ok(None)
}

fn validate_percent_escapes(resource: &str) -> Result<(), ResourceError> {
    let mut bytes = resource.as_bytes().iter();
    while let Some(byte) = bytes.next() {
        if *byte != b'%' {
            continue;
        }
        let Some(high) = bytes.next() else {
            return Err(ResourceError::InvalidPercentEncoding);
        };
        let Some(low) = bytes.next() else {
            return Err(ResourceError::InvalidPercentEncoding);
        };
        if !high.is_ascii_hexdigit() || !low.is_ascii_hexdigit() {
            return Err(ResourceError::InvalidPercentEncoding);
        }
    }
    Ok(())
}

fn validate_uri_characters(resource: &str) -> Result<(), ResourceError> {
    if resource.bytes().all(is_uri_character) {
        Ok(())
    } else {
        Err(ResourceError::InvalidCharacters)
    }
}

fn is_uri_character(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b':'
            | b'/'
            | b'?'
            | b'#'
            | b'['
            | b']'
            | b'@'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'%'
    )
}

fn scheme(resource: &str) -> Option<&str> {
    let mut bytes = resource.bytes();
    let first = bytes.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }

    for (index, byte) in bytes.enumerate() {
        match byte {
            b':' => return Some(&resource[..index + 1]),
            b'/' | b'?' | b'#' => return None,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'-' | b'.' => {}
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Accepts `acct:` resources because they are absolute URIs with a scheme.
    #[test]
    fn accepts_acct_resource() {
        let resource = "acct:carol@example.com".parse::<Resource>().unwrap();

        assert_eq!(resource.as_str(), "acct:carol@example.com");
    }

    /// Accepts hierarchical HTTPS resources with an authority.
    #[test]
    fn accepts_https_resource() {
        let resource = "https://example.org/users/carol"
            .parse::<Resource>()
            .unwrap();

        assert_eq!(resource.as_str(), "https://example.org/users/carol");
        assert_eq!(resource.host(), Some("example.org"));
    }

    /// Accepts scheme-specific opaque-looking URIs.
    ///
    /// RFC 3986's `URI` production requires a scheme but allows a scheme-specific path without an
    /// authority. WebFinger commonly uses this shape for `acct:` resources.
    #[test]
    fn accepts_scheme_specific_resource() {
        let resource = "urn:example:animal:ferret:nose"
            .parse::<Resource>()
            .unwrap();

        assert_eq!(resource.as_str(), "urn:example:animal:ferret:nose");
    }

    /// Rejects relative references that `http::Uri` can otherwise parse.
    #[test]
    fn rejects_relative_resource_references() {
        for resource in ["carol", "/relative", "../x", ""] {
            let error = resource.parse::<Resource>().unwrap_err();

            assert!(
                matches!(error, ResourceError::RelativeReference),
                "expected relative-resource error for {resource:?}, got {error:?}",
            );
        }
    }

    /// Rejects raw non-ASCII resource text.
    ///
    /// RFC 3986 URI syntax is ASCII. Non-ASCII data must be percent-encoded inside the resource URI
    /// itself before it is put into the WebFinger query parameter.
    #[test]
    fn rejects_non_ascii_resource_text() {
        let error = "acct:carolé@example.org".parse::<Resource>().unwrap_err();

        assert!(
            matches!(error, ResourceError::InvalidCharacters),
            "expected invalid-character error, got {error:?}",
        );
    }

    /// Rejects raw ASCII characters outside the RFC 3986 URI character set.
    #[test]
    fn rejects_invalid_raw_uri_characters() {
        for resource in [
            "acct:carol{bad}@example.org",
            "acct:carol|bad@example.org",
            "acct:carol^bad@example.org",
            "acct:carol`bad@example.org",
        ] {
            let error = resource.parse::<Resource>().unwrap_err();

            assert!(
                matches!(error, ResourceError::InvalidCharacters),
                "expected invalid-character error for {resource:?}, got {error:?}",
            );
        }
    }

    /// Accepts characters outside the raw URI character set when they are percent-encoded.
    #[test]
    fn accepts_percent_encoded_invalid_raw_characters() {
        let resource = "acct:carol%7Bbad%7D@example.org"
            .parse::<Resource>()
            .unwrap();

        assert_eq!(resource.as_str(), "acct:carol%7Bbad%7D@example.org");
    }

    /// Rejects malformed percent escape syntax inside resource URIs.
    ///
    /// Percent escapes belong to the resource URI itself after the outer WebFinger query has been
    /// decoded, so malformed escapes must be rejected at the resource boundary too.
    #[test]
    fn rejects_malformed_resource_percent_escape() {
        let error = "acct:carol%GG@example.org".parse::<Resource>().unwrap_err();

        assert!(
            matches!(error, ResourceError::InvalidPercentEncoding),
            "expected invalid-percent-encoding error, got {error:?}",
        );
    }

    /// Rejects HTTP and HTTPS resources that omit the required authority.
    #[test]
    fn rejects_http_resources_without_authority() {
        for resource in ["http:foo", "https:foo", "http:/example.org/path"] {
            let error = resource.parse::<Resource>().unwrap_err();

            assert!(
                matches!(error, ResourceError::MissingHttpAuthority),
                "expected missing-authority error for {resource:?}, got {error:?}",
            );
        }
    }

    /// Validates HTTP and HTTPS resource authorities regardless of scheme case.
    ///
    /// URI schemes are case-insensitive, so uppercase `HTTPS` should not bypass the stricter
    /// hierarchical URI validation used for HTTP resources.
    #[test]
    fn rejects_invalid_https_authority_with_uppercase_scheme() {
        let error = "HTTPS://[::1".parse::<Resource>().unwrap_err();

        assert!(
            matches!(error, ResourceError::InvalidHttpUri(_)),
            "expected invalid-authority error, got {error:?}",
        );
    }
}
