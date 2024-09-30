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
