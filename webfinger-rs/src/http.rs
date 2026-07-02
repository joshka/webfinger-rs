use std::str::FromStr;

use http::Uri;
use http::uri::{InvalidUri, PathAndQuery, Scheme};
use percent_encoding::{AsciiSet, utf8_percent_encode};

use crate::{WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};

pub(crate) const CORS_ALLOW_ORIGIN: &str = "*";

/// The set of values to percent encode
///
/// Notably, this set does not include the `@`, `:`, `?`, and `/` characters which are allowed by
/// RFC 3986 in the query component. It does include `%` so already-percent-encoded resource or
/// relation URIs survive WebFinger query parsing as literal percent escapes in the target value.
///
/// See the following RFCs for more information:
/// - <https://www.rfc-editor.org/rfc/rfc7033#section-4.1>
/// - <https://www.rfc-editor.org/rfc/rfc3986#section-2.1>
/// - <https://www.rfc-editor.org/rfc/rfc3986#section-3.4>
/// - <https://www.rfc-editor.org/rfc/rfc3986#appendix-A>
///
/// Note: this may be implemented in the `percent-encoding` crate soon in
/// <https://github.com/servo/rust-url/pull/971>
const QUERY: AsciiSet = percent_encoding::CONTROLS
    // RFC 3986
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    // RFC 7033
    .add(b'=')
    .add(b'&');

impl TryFrom<&WebFingerRequest> for PathAndQuery {
    type Error = InvalidUri;

    fn try_from(query: &WebFingerRequest) -> Result<PathAndQuery, InvalidUri> {
        let resource = query.resource.to_string();
        let resource = utf8_percent_encode(&resource, &QUERY).to_string();
        let mut path = WELL_KNOWN_PATH.to_owned();
        path.push_str("?resource=");
        path.push_str(&resource);
        for rel in &query.rels {
            let rel = utf8_percent_encode(rel.as_ref(), &QUERY).to_string();
            path.push_str("&rel=");
            path.push_str(&rel);
        }
        PathAndQuery::from_str(&path)
    }
}

impl TryFrom<&WebFingerRequest> for Uri {
    type Error = http::Error;

    fn try_from(query: &WebFingerRequest) -> Result<Uri, http::Error> {
        let path_and_query = PathAndQuery::try_from(query)?;

        // HTTPS is mandatory
        // <https://www.rfc-editor.org/rfc/rfc7033.html#section-4>
        // <https://www.rfc-editor.org/rfc/rfc7033.html#section-9.1>
        const SCHEME: Scheme = Scheme::HTTPS;

        Uri::builder()
            .scheme(SCHEME)
            .authority(query.host.clone())
            .path_and_query(path_and_query)
            .build()
    }
}

impl TryFrom<&WebFingerResponse> for http::Response<()> {
    type Error = http::Error;
    fn try_from(_: &WebFingerResponse) -> Result<http::Response<()>, http::Error> {
        http::Response::builder()
            .header("Content-Type", "application/jrd+json")
            .header("Access-Control-Allow-Origin", CORS_ALLOW_ORIGIN)
            .body(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rel;

    /// Percent-encodes literal percent signs in the outgoing `resource` value.
    ///
    /// RFC 7033 section 4.1 puts the resource URI inside a WebFinger query parameter. If the target
    /// URI already contains percent escapes, the `%` signs must become `%25` in the outer query or a
    /// server will decode them as part of the WebFinger parameter and change the target resource.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn outgoing_resource_preserves_inner_percent_escapes() {
        let request = WebFingerRequest {
            resource: "https://example.org/profile/a%20b".parse().unwrap(),
            host: "example.org".to_string(),
            rels: Vec::new(),
        };

        let uri = Uri::try_from(&request).unwrap();

        assert_eq!(
            uri.to_string(),
            "https://example.org/.well-known/webfinger?resource=https://example.org/profile/a%2520b",
        );
    }

    /// Percent-encodes literal percent signs in outgoing `rel` values.
    ///
    /// Relation filters are also WebFinger query parameter values. Encoding `%` prevents an already
    /// escaped relation URI from being decoded one level too far by the receiving WebFinger server.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn outgoing_rel_preserves_inner_percent_escapes() {
        let request = WebFingerRequest {
            resource: "acct:carol@example.org".parse().unwrap(),
            host: "example.org".to_string(),
            rels: vec![Rel::new("https://example.org/rel/a%2Fb")],
        };

        let uri = Uri::try_from(&request).unwrap();

        assert_eq!(
            uri.to_string(),
            "https://example.org/.well-known/webfinger?resource=acct:carol@example.org&rel=https://example.org/rel/a%252Fb",
        );
    }
}
