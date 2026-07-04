use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;
use webfinger_rs::{JrdUri, Link, Rel, WebFingerRequest, WebFingerResponse};

/// WebFinger resources loaded from TOML configuration.
///
/// `Config` is the in-memory representation used by [`StaticConfigProvider`](crate::StaticConfigProvider)
/// and by runtime providers that load TOML from another store before resolving a request.
///
/// Resources are keyed by their exact `resource` string. A request for
/// `acct:alice@example.com` does not match `acct:Alice@example.com`, an alias URL, or any inferred
/// account domain. Relation filtering is applied during resolution: when a request contains one or
/// more `rel` parameters, the returned response contains only links with matching relation values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    resources: BTreeMap<String, WebFingerResponse>,
}

impl Config {
    /// Parses WebFinger configuration from TOML.
    ///
    /// The top-level TOML document must contain a `resources` array. Each resource maps onto a JRD
    /// response with supported resource-level fields `resource`, `aliases`, and `properties`, and
    /// supported link-level fields `rel`, `type`, `href`, `template`, `titles`, and `properties`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when the TOML is malformed, contains duplicate resource entries,
    /// uses unsupported fields, uses an invalid WebFinger/JRD URI value, or uses the `{ null =
    /// true }` property marker incorrectly.
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(input)?;
        let mut resources = BTreeMap::new();
        for resource in raw.resources {
            let response = resource.into_response()?;
            let previous = resources.insert(response.subject.to_string(), response);
            if let Some(previous) = previous {
                return Err(ConfigError::DuplicateResource(previous.subject.to_string()));
            }
        }
        Ok(Self { resources })
    }

    /// Resolves a request against the configured resources.
    ///
    /// Returns `None` when the requested resource is not present. Returned responses are cloned
    /// from the config so relation filtering can remove links without mutating shared
    /// configuration.
    pub fn resolve(&self, request: &WebFingerRequest) -> Option<WebFingerResponse> {
        let response = self.resources.get(request.resource.as_ref())?;
        Some(filter_response(response.clone(), &request.rels))
    }
}

fn filter_response(mut response: WebFingerResponse, rels: &[Rel]) -> WebFingerResponse {
    if !rels.is_empty() {
        response.links.retain(|link| rels.contains(&link.rel));
    }
    response
}

/// Errors raised while parsing WebFinger TOML configuration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The TOML was malformed.
    #[error("invalid TOML configuration: {0}")]
    Toml(#[from] toml::de::Error),

    /// A configured resource appeared more than once.
    #[error("duplicate resource `{0}`")]
    DuplicateResource(String),

    /// A configured resource or JRD URI field was invalid.
    #[error(transparent)]
    WebFinger(#[from] webfinger_rs::Error),

    /// A configured property used the TOML null marker incorrectly.
    #[error("property `{0}` uses invalid null marker; use `{{ null = true }}`")]
    InvalidNullProperty(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    resources: Vec<RawResource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResource {
    resource: String,
    aliases: Option<Vec<String>>,
    properties: Option<BTreeMap<String, RawPropertyValue>>,
    links: Option<Vec<RawLink>>,
}

impl RawResource {
    fn into_response(self) -> Result<WebFingerResponse, ConfigError> {
        let mut builder = WebFingerResponse::try_builder(&self.resource)?;
        if let Some(aliases) = self.aliases {
            for alias in aliases {
                builder = builder.alias(alias);
            }
        }
        if let Some(properties) = self.properties {
            for (key, value) in properties {
                builder = match value.into_option(&key)? {
                    Some(value) => builder.property(key, value),
                    None => builder.null_property(key),
                };
            }
        }
        if let Some(links) = self.links {
            let links = links
                .into_iter()
                .map(RawLink::into_link)
                .collect::<Result<Vec<_>, _>>()?;
            builder = builder.links(links);
        }
        Ok(builder.build())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLink {
    rel: String,
    r#type: Option<String>,
    href: Option<String>,
    template: Option<String>,
    titles: Option<BTreeMap<String, String>>,
    properties: Option<BTreeMap<String, RawPropertyValue>>,
}

impl RawLink {
    fn into_link(self) -> Result<Link, ConfigError> {
        let mut link = Link::new(Rel::try_new(self.rel)?);
        link.r#type = self.r#type;
        link.href = self.href.map(JrdUri::try_new).transpose()?;
        link.template = self.template;
        link.titles = self.titles;
        link.properties = self.properties.map(parse_properties).transpose()?;
        Ok(link)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawPropertyValue {
    String(String),
    Null { null: bool },
}

impl RawPropertyValue {
    fn into_option(self, key: &str) -> Result<Option<String>, ConfigError> {
        match self {
            RawPropertyValue::String(value) => Ok(Some(value)),
            RawPropertyValue::Null { null } => {
                if null {
                    Ok(None)
                } else {
                    Err(ConfigError::InvalidNullProperty(key.to_string()))
                }
            }
        }
    }
}

fn parse_properties(
    properties: BTreeMap<String, RawPropertyValue>,
) -> Result<BTreeMap<JrdUri, Option<String>>, ConfigError> {
    properties
        .into_iter()
        .map(|(key, value)| {
            let property = JrdUri::try_new(key)?;
            let value = value.into_option(property.as_ref())?;
            Ok((property, value))
        })
        .collect()
}
