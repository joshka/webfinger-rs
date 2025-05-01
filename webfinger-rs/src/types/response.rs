use std::collections::HashMap;
use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::Rel;

/// A WebFinger response.
///
/// This represents the response portion of a WebFinger query that is returned by a WebFinger
/// server.
///
/// See [RFC 7033 Section 4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4) for more
/// information.
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::{Link, WebFingerResponse};
///
/// let avatar = Link::builder("http://webfinger.net/rel/avatar")
///     .href("https://example.com/avatar.png")
///     .build();
/// let profile = Link::builder("http://webfinger.net/rel/profile-page")
///     .href("https://example.com/profile/carol")
///     .build();
/// let response = WebFingerResponse::builder("acct:carol@example.com")
///     .alias("https://example.com/profile/carol")
///     .property("https://example.com/ns/role", "developer")
///     .link(avatar)
///     .link(profile)
///     .build();
/// ```
///
/// `Response` can be used as a response in Axum handlers as it implements
/// [`axum::response::IntoResponse`].
///
/// ```rust
/// use axum::response::IntoResponse;
/// use webfinger_rs::{Link, WebFingerRequest, WebFingerResponse};
///
/// async fn handler(request: WebFingerRequest) -> WebFingerResponse {
///     // ... handle the request ...
///     WebFingerResponse::builder("acct:carol@example.com")
///         .alias("https://example.com/profile/carol")
///         .property("https://example.com/ns/role", "developer")
///         .link(
///             Link::builder("http://webfinger.net/rel/avatar")
///                 .href("https://example.com/avatar.png"),
///         )
///         .build()
/// }
/// ```
#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Response {
    /// The subject of the response.
    ///
    /// This is the URI of the resource that the response is about.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.1)
    pub subject: String,

    /// The aliases of the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.2](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.2)
    pub aliases: Option<Vec<String>>,

    /// The properties of the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.3](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3)
    pub properties: Option<HashMap<String, String>>,

    /// The links of the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4)
    pub links: Vec<Link>,
}

impl Response {
    /// Create a new response with the given subject.
    pub fn new<S: Into<String>>(subject: S) -> Self {
        Self {
            subject: subject.into(),
            aliases: None,
            properties: None,
            links: Vec::new(),
        }
    }

    /// Create a new [`WebFingerBuilder`] with the given subject.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use webfinger_rs::{Link, WebFingerResponse};
    ///
    /// let avatar =
    ///     Link::builder("http://webfinger.net/rel/avatar").href("https://example.com/avatar.png");
    /// let response = WebFingerResponse::builder("acct:carol@example.com")
    ///     .alias("https://example.com/profile/carol")
    ///     .property("https://example.com/ns/role", "developer")
    ///     .link(avatar)
    ///     .build();
    /// ```
    pub fn builder<S: Into<String>>(subject: S) -> Builder {
        Builder::new(subject.into())
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string_pretty(self).unwrap())
    }
}

/// A builder for a WebFinger response.
///
/// This is used to construct a [`Response`] with the desired fields.
pub struct Builder {
    response: Response,
}

impl Builder {
    /// Create a new response builder with the given subject.
    pub fn new<S: Into<String>>(subject: S) -> Self {
        Self {
            response: Response::new(subject.into()),
        }
    }

    /// Add an alias to the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.2](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.2)
    pub fn alias<S: Into<String>>(mut self, alias: S) -> Self {
        self.response
            .aliases
            .get_or_insert_with(Vec::new)
            .push(alias.into());
        self
    }

    /// Add a property to the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.3](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3)
    pub fn property<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.response
            .properties
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Add a link to the response.
    ///
    /// If the link is constructed with a builder, it is not necessary to call the `build` method on
    /// the link as the builder implements `From<LinkBuilder> for Link`.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4)
    pub fn link<L: Into<Link>>(mut self, link: L) -> Self {
        self.response.links.push(link.into());
        self
    }

    /// Set the links of the response.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4)
    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.response.links = links;
        self
    }

    /// Build the response.
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

/// A link in the WebFinger response.
///
/// Defined in [RFC 7033 Section 4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4)
#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct Link {
    /// The relation type of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.1)
    pub rel: Rel,

    /// The media type of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.2](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.2)
    pub r#type: Option<String>,

    /// The target URI of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.3](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.3)
    pub href: Option<String>,

    /// The titles of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4)
    pub titles: Option<Vec<Title>>,

    /// The properties of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.5](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5)
    pub properties: Option<HashMap<String, Option<String>>>,
}

impl Link {
    /// Create a new link with the given relation type.
    pub fn new(rel: Rel) -> Self {
        Self {
            rel,
            r#type: None,
            href: None,
            titles: None,
            properties: None,
        }
    }

    /// Create a new [`LinkBuilder`] with the given relation type.
    pub fn builder<R: Into<Rel>>(rel: R) -> LinkBuilder {
        LinkBuilder::new(rel)
    }
}

/// A builder for a WebFinger link.
///
/// This is used to construct a [`Link`] with the desired fields.
pub struct LinkBuilder {
    link: Link,
}

impl LinkBuilder {
    /// Create a new link builder with the given relation type.
    pub fn new<R: Into<Rel>>(rel: R) -> Self {
        Self {
            link: Link::new(rel.into()),
        }
    }

    /// Set the media type of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.2](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.2)
    pub fn r#type<S: Into<String>>(mut self, r#type: S) -> Self {
        self.link.r#type = Some(r#type.into());
        self
    }

    /// Set the target URI of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.3](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.3)
    pub fn href<S: Into<String>>(mut self, href: S) -> Self {
        self.link.href = Some(href.into());
        self
    }

    /// Add a single title for the the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4)
    pub fn title<L: Into<String>, V: Into<String>>(mut self, language: L, value: V) -> Self {
        let title = Title::new(language, value);
        self.link.titles.get_or_insert_with(Vec::new).push(title);
        self
    }

    /// Set the titles of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4)
    pub fn titles(mut self, titles: Vec<Title>) -> Self {
        self.link.titles = Some(titles);
        self
    }

    /// Add a single property to the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.5](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5)
    pub fn property<K: Into<String>, V: Into<Option<String>>>(mut self, key: K, value: V) -> Self {
        self.link
            .properties
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Set the properties of the link.
    ///
    /// Defined in [RFC 7033 Section
    /// 4.4.4.5](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5)
    pub fn properties(mut self, properties: HashMap<String, Option<String>>) -> Self {
        self.link.properties = Some(properties);
        self
    }

    /// Build the link.
    ///
    /// This can be omitted if the link is being converted to a `Link` directly from the builder as
    /// `LinkBuilder` also implements `From<LinkBuilder> for Link`.
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

/// A title in the WebFinger response.
///
/// Defined in [RFC 7033 Section 4.4.4.4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4)
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::Title;
///
/// let title = Title::new("en-us", "Carol's Profile");
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Title {
    /// The language of the title.
    ///
    /// This can be any valid language tag as defined in [RFC
    /// 5646](https://www.rfc-editor.org/rfc/rfc5646.html) or the string `und` to indicate an
    /// undefined language.
    pub language: String,
    /// The value of the title.
    pub value: String,
}

impl Title {
    /// Create a new title with the given language and value.
    pub fn new<L: Into<String>, V: Into<String>>(language: L, value: V) -> Self {
        Self {
            language: language.into(),
            value: value.into(),
        }
    }
}
