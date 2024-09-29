use std::{collections::HashMap, fmt::Debug};

use ::http::{uri::Authority, Uri};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

mod http;

#[cfg(feature = "reqwest")]
mod reqwest;

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

// JSON Resource Descriptor (JRD)
#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub subject: String,
    pub aliases: Option<Vec<String>>,
    pub properties: Option<HashMap<String, String>>,
    pub links: Vec<Link>,
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
    pub rel: LinkRelationType,
    pub r#type: Option<String>,
    pub href: Option<String>,
    pub titles: Option<Vec<Title>>,
    pub properties: Option<HashMap<String, Option<String>>>,
}

impl Link {
    pub fn new(rel: LinkRelationType) -> Self {
        Self {
            rel,
            r#type: None,
            href: None,
            titles: None,
            properties: None,
        }
    }
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

/// Link relation type
///
/// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.1>
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

impl From<String> for LinkRelationType {
    fn from(s: String) -> LinkRelationType {
        LinkRelationType(s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] ::http::Error),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] ::reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use ::http::Uri;

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
