use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::Rel;

// JSON Resource Descriptor (JRD)
#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub subject: String,
    pub aliases: Option<Vec<String>>,
    pub properties: Option<HashMap<String, String>>,
    pub links: Vec<Link>,
}

impl Response {
    pub fn new<S: Into<String>>(subject: S) -> Self {
        Self {
            subject: subject.into(),
            aliases: None,
            properties: None,
            links: Vec::new(),
        }
    }

    pub fn builder<S: Into<String>>(subject: S) -> Builder {
        Builder::new(subject.into())
    }
}

pub struct Builder {
    response: Response,
}

impl Builder {
    pub fn new(subject: String) -> Self {
        Self {
            response: Response::new(subject),
        }
    }

    pub fn alias(mut self, alias: String) -> Self {
        self.response
            .aliases
            .get_or_insert_with(Vec::new)
            .push(alias);
        self
    }

    pub fn property(mut self, key: String, value: String) -> Self {
        self.response
            .properties
            .get_or_insert_with(HashMap::new)
            .insert(key, value);
        self
    }

    pub fn link<L: Into<Link>>(mut self, link: L) -> Self {
        self.response.links.push(link.into());
        self
    }

    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.response.links = links;
        self
    }

    pub fn build(self) -> Response {
        self.response
    }
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

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Link {
    pub rel: Rel,
    pub r#type: Option<String>,
    pub href: Option<String>,
    pub titles: Option<Vec<Title>>,
    pub properties: Option<HashMap<String, Option<String>>>,
}

impl Link {
    pub fn new(rel: Rel) -> Self {
        Self {
            rel,
            r#type: None,
            href: None,
            titles: None,
            properties: None,
        }
    }

    pub fn builder<R: Into<Rel>>(rel: R) -> LinkBuilder {
        LinkBuilder::new(rel)
    }
}

pub struct LinkBuilder {
    link: Link,
}

impl LinkBuilder {
    pub fn new<R: Into<Rel>>(rel: R) -> Self {
        Self {
            link: Link::new(rel.into()),
        }
    }

    pub fn r#type<S: Into<String>>(mut self, r#type: S) -> Self {
        self.link.r#type = Some(r#type.into());
        self
    }

    pub fn href<S: Into<String>>(mut self, href: S) -> Self {
        self.link.href = Some(href.into());
        self
    }

    pub fn title<L: Into<String>, V: Into<String>>(mut self, language: L, value: V) -> Self {
        self.link.titles.get_or_insert_with(Vec::new).push(Title {
            language: language.into(),
            value: value.into(),
        });
        self
    }

    pub fn titles(mut self, titles: Vec<Title>) -> Self {
        self.link.titles = Some(titles);
        self
    }

    pub fn property<K: Into<String>, V: Into<Option<String>>>(mut self, key: K, value: V) -> Self {
        self.link
            .properties
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    pub fn properties(mut self, properties: HashMap<String, Option<String>>) -> Self {
        self.link.properties = Some(properties);
        self
    }

    pub fn build(self) -> Link {
        self.link
    }
}

impl From<LinkBuilder> for Link {
    fn from(builder: LinkBuilder) -> Self {
        builder.build()
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
    pub(crate) language: String,
    pub(crate) value: String,
}
