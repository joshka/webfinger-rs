use std::str::FromStr;

use http::uri::{InvalidUri, PathAndQuery, Scheme};
use http::Uri;
use percent_encoding::{utf8_percent_encode, AsciiSet};

use crate::{WebFingerRequest, WebFingerResponse, WELL_KNOWN_PATH};

/// The set of values to percent encode
///
/// Notably, this set does not include the `@`, `:`, `?`, and `/` characters which are allowed by
/// RFC 3986 in the query component.
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
            let rel = utf8_percent_encode(rel, &QUERY).to_string();
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
            .body(())
    }
}
