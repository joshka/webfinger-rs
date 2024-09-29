use std::{collections::HashMap, fmt::Debug, str::FromStr};

use http::{
    uri::{Authority, InvalidUri, PathAndQuery, Scheme},
    Uri,
};
use percent_encoding::{utf8_percent_encode, AsciiSet};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::debug;

const WELL_KNOWN_PATH: &str = "/.well-known/webfinger";
#[allow(unused)]
const JRD_CONTENT_TYPE: &str = "application/jrd+json";

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

#[derive(Debug)]
pub struct Request {
    /// The host to query
    ///
    /// TODO: This might actually be just the host name, not the full authority.
    pub host: Authority,

    /// Query target.
    ///
    /// This is the URI of the resource to query. It will be stored in the `resource` query
    /// parameter.
    pub resource: Uri,

    /// Link relation types
    ///
    /// This is a list of link relation types to query for. Each link relation type will be stored
    /// in a `rel` query parameter.
    pub link_relation_types: Vec<LinkRelationType>,
}

impl TryFrom<&Request> for PathAndQuery {
    type Error = InvalidUri;

    fn try_from(query: &Request) -> Result<PathAndQuery, InvalidUri> {
        let resource = query.resource.to_string();
        let resource = utf8_percent_encode(&resource, &QUERY).to_string();
        let mut path = WELL_KNOWN_PATH.to_owned();
        path.push_str("?resource=");
        path.push_str(&resource);
        for rel in &query.link_relation_types {
            let rel = utf8_percent_encode(&rel.0, &QUERY).to_string();
            path.push_str("&rel=");
            path.push_str(&rel);
        }
        PathAndQuery::from_str(&path)
    }
}

impl TryFrom<&Request> for Uri {
    type Error = http::Error;

    fn try_from(query: &Request) -> Result<Uri, http::Error> {
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

struct EmptyBody;

#[cfg(feature = "reqwest")]
impl From<EmptyBody> for reqwest::Body {
    fn from(_: EmptyBody) -> reqwest::Body {
        reqwest::Body::default()
    }
}

impl TryFrom<&Request> for http::Request<EmptyBody> {
    type Error = http::Error;

    fn try_from(query: &Request) -> Result<http::Request<EmptyBody>, http::Error> {
        let uri = Uri::try_from(query)?;
        http::Request::builder()
            .method("GET")
            .uri(uri)
            .body(EmptyBody)
    }
}

// JSON Resource Descriptor (JRD)
#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Response {
    subject: String,
    aliases: Option<Vec<String>>,
    properties: Option<HashMap<String, String>>,
    links: Vec<Link>,
}

/// Custom debug implementation to avoid printing `None` fields
impl Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Response");
        let mut debug = debug.field("subject", &self.subject);
        if let Some(aliases) = &self.aliases {
            debug = debug.field("aliases", &aliases);
        }
        if let Some(properties) = &self.properties {
            debug = debug.field("properties", &properties);
        }
        debug.field("links", &self.links).finish()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Link {
    rel: LinkRelationType,
    r#type: Option<String>,
    href: Option<String>,
    titles: Option<Vec<Title>>,
    properties: Option<HashMap<String, Option<String>>>,
}

/// Custom debug implementation to avoid printing `None` fields
impl Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Link");
        let mut debug = debug.field("rel", &self.rel);
        if let Some(r#type) = &self.r#type {
            debug = debug.field("type", &r#type);
        }
        if let Some(href) = &self.href {
            debug = debug.field("href", &href);
        }
        if let Some(titles) = &self.titles {
            debug = debug.field("titles", &titles);
        }
        if let Some(properties) = &self.properties {
            debug = debug.field("properties", &properties);
        }
        debug.finish()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Title {
    language: String,
    value: String,
}

impl TryFrom<&Response> for http::Response<()> {
    type Error = http::Error;
    fn try_from(_: &Response) -> Result<http::Response<()>, http::Error> {
        http::Response::builder()
            .header("Content-Type", "application/jrd+json")
            .body(())
    }
}

/// Link relation type
#[derive(Serialize, Deserialize)]
pub struct LinkRelationType(String);

impl Debug for LinkRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for LinkRelationType {
    fn from(s: &str) -> LinkRelationType {
        LinkRelationType(s.to_owned())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] http::Error),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(feature = "reqwest")]
impl Request {
    #[tracing::instrument]
    pub async fn fetch(&self) -> Result<Response, Error> {
        let client = reqwest::Client::new();
        let request = http::Request::try_from(self)?;
        let request = reqwest::Request::try_from(request)?;
        let response = client.execute(request).await?;
        debug!("response: {:?}", response);
        let response = response.error_for_status()?;
        let body = response.text().await?;
        debug!(body, "response body");
        let response = serde_json::from_str(&body)?;
        // let response = response.json().await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::*;

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.1
    #[test]
    fn example_3_1() {
        let resource = "acct:carol@example.com".parse().unwrap();
        let rel = LinkRelationType::from("http://openid.net/specs/connect/1.0/issuer");
        let host = "example.com".parse().unwrap();
        let query = Request {
            host,
            resource,
            link_relation_types: vec![rel],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // `"/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fopenid.net%2Fspecs%2Fconnect%2F1.0%2Fissuer"`
        assert_eq!(
            uri.to_string(),
            "/.well-known/webfinger?resource=acct:carol@example.com&rel=http://openid.net/specs/connect/1.0/issuer",
            );
    }

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.2
    #[test]
    fn example_3_2() {
        let resource = "http://blog.example.com/article/id/314".parse().unwrap();
        let query = Request {
            host: "blog.example.com".parse().unwrap(),
            resource,
            link_relation_types: vec![],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // /.well-known/webfinger?resource=http%3A%2F%2Fblog.example.com%2Farticle%2Fid%2F314
        assert_eq!(
            uri.to_string(),
            "/.well-known/webfinger?resource=http://blog.example.com/article/id/314",
        );
    }
}
