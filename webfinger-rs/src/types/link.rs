use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{JrdUri, Rel};

/// A link in the WebFinger response.
///
/// Link objects describe related resources for the JRD subject. RFC 7033 gives each link a
/// required [`rel`](Self::rel) member and optional `type`, `href`, `titles`, and `properties`
/// members. Some WebFinger profiles also use the JRD `template` member from RFC 6415 link
/// templates.
///
/// The Rust fields mirror the JRD JSON shape:
///
/// - [`rel`](Self::rel) is a [`Rel`] so the required relation string is validated as one relation
///   type.
/// - [`href`](Self::href) is a [`JrdUri`] because RFC 7033 defines it as a URI string.
/// - [`template`](Self::template) is a URI template string.
/// - [`titles`](Self::titles) is a language-keyed object, matching the RFC JSON form.
/// - [`properties`](Self::properties) uses [`JrdUri`] keys and `Option<String>` values so JSON
///   `null` is representable.
///
/// Use [`Link::builder`] for ordinary construction from string literals or application values. Use
/// [`Link::new`] when you already have a validated [`Rel`].
///
/// See [RFC 7033 section 4.4.4].
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::Link;
///
/// let link = Link::builder("http://webfinger.net/rel/profile-page")
///     .href("https://example.com/profile/carol")
///     .r#type("text/html")
///     .title("en-us", "Carol's profile")
///     .property("https://example.com/ns/verified", "true")
///     .null_property("https://example.com/ns/old-profile")
///     .build();
///
/// assert_eq!(link.rel.as_ref(), "http://webfinger.net/rel/profile-page");
/// ```
///
/// [RFC 7033 section 4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Link {
    /// The relation type of the link.
    ///
    /// This member is required by [RFC 7033 section 4.4.4.1]. It uses [`Rel`] instead of `String`
    /// so deserialization and builder construction both reject empty or malformed relation
    /// values.
    ///
    /// [RFC 7033 section 4.4.4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.1
    pub rel: Rel,

    /// The media type of the link.
    ///
    /// RFC 7033 leaves this as a media type string. The crate stores it as `String` because it is
    /// advisory metadata for the linked representation, not one of the WebFinger URI-valued
    /// fields.
    ///
    /// See [RFC 7033 section 4.4.4.2].
    ///
    /// [RFC 7033 section 4.4.4.2]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.2
    pub r#type: Option<String>,

    /// The target URI of the link.
    ///
    /// RFC 7033 defines `href` as a URI string. The field uses [`JrdUri`] rather than `String` so
    /// relative references are rejected when links are deserialized or built through the builder.
    ///
    /// See [RFC 7033 section 4.4.4.3].
    ///
    /// [RFC 7033 section 4.4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.3
    pub href: Option<JrdUri>,

    /// A URI template for the link.
    ///
    /// RFC 6415 defines `template` as an optional JRD link member for link templates. The crate
    /// stores it as a string because WebFinger servers do not need to parse or expand the template
    /// expression before serializing it.
    ///
    /// See [RFC 6415 appendix A].
    ///
    /// [RFC 6415 appendix A]: https://www.rfc-editor.org/rfc/rfc6415.html#appendix-A
    pub template: Option<String>,

    /// The titles of the link.
    ///
    /// RFC 7033 models titles as a JSON object whose keys are language tags and whose values are
    /// title strings. The crate uses a `BTreeMap` so direct struct construction preserves that JSON
    /// object shape and gets deterministic ordering for comparisons, hashing, and rendered output.
    ///
    /// Use [`LinkBuilder::title`] for one title at a time or [`LinkBuilder::titles`] to set a full
    /// language map.
    ///
    /// See [RFC 7033 section 4.4.4.4].
    ///
    /// [RFC 7033 section 4.4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4
    pub titles: Option<BTreeMap<String, String>>,

    /// The properties of the link.
    ///
    /// Link properties are a JSON object whose property identifiers are URI strings. Values may be
    /// strings or JSON `null`, so the Rust value type is `Option<String>`. `None` serializes as a
    /// property value of `null`; it does not omit the property from the map.
    ///
    /// Use [`LinkBuilder::property`] for string-valued properties and
    /// [`LinkBuilder::null_property`] for JSON `null` values.
    ///
    /// See [RFC 7033 section 4.4.4.5].
    ///
    /// [RFC 7033 section 4.4.4.5]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5
    pub properties: Option<BTreeMap<JrdUri, Option<String>>>,
}

impl Link {
    /// Creates a link from an already validated relation type.
    ///
    /// The returned link has no optional members set. This is useful when relation validation
    /// happens separately, for example when reusing a [`Rel`] from a request filter. Use
    /// [`Link::builder`] when constructing a link directly from strings.
    pub fn new(rel: Rel) -> Self {
        Self {
            rel,
            r#type: None,
            href: None,
            template: None,
            titles: None,
            properties: None,
        }
    }

    /// Creates a [`LinkBuilder`] with the given relation type.
    ///
    /// The builder accepts a string-like value for the common case and validates it into [`Rel`].
    /// Invalid values panic through [`Rel::new`]; use [`Rel::try_new`] and [`Link::new`] when the
    /// relation comes from untrusted input.
    pub fn builder<R: AsRef<str>>(rel: R) -> LinkBuilder {
        LinkBuilder::new(rel)
    }
}

/// A builder for a WebFinger link.
///
/// `LinkBuilder` keeps common JRD construction concise while preserving the typed representation
/// used by [`Link`]. String arguments are accepted at the method boundary and converted into
/// [`Rel`] or [`JrdUri`] where the RFC requires those shapes.
///
/// The builder can be passed directly to [`ResponseBuilder::link`](crate::ResponseBuilder::link)
/// because `Link` implements `From<LinkBuilder>`.
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::{Link, WebFingerResponse};
///
/// let response = WebFingerResponse::builder("acct:carol@example.com")
///     .link(
///         Link::builder("author")
///             .href("https://example.com/people/carol")
///             .title("en-us", "Carol"),
///     )
///     .build();
///
/// assert_eq!(response.links[0].rel.as_ref(), "author");
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkBuilder {
    link: Link,
}

impl LinkBuilder {
    /// Creates a link builder with the given relation type.
    ///
    /// The relation is validated immediately. This catches invalid builder input before the
    /// response is serialized.
    pub fn new<R: AsRef<str>>(rel: R) -> Self {
        Self {
            link: Link::new(Rel::new(rel)),
        }
    }

    /// Sets the media type of the link.
    ///
    /// This writes the optional `type` member from [RFC 7033 section 4.4.4.2].
    ///
    /// [RFC 7033 section 4.4.4.2]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.2
    pub fn r#type<S: Into<String>>(mut self, r#type: S) -> Self {
        self.link.r#type = Some(r#type.into());
        self
    }

    /// Sets the target URI of the link.
    ///
    /// The value is validated as a [`JrdUri`] and serialized as the optional `href` member from
    /// [RFC 7033 section 4.4.4.3].
    ///
    /// [RFC 7033 section 4.4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.3
    pub fn href<S: AsRef<str>>(mut self, href: S) -> Self {
        self.link.href = Some(JrdUri::new(href));
        self
    }

    /// Sets a URI template for the link.
    ///
    /// This writes the optional JRD `template` member from [RFC 6415 appendix A].
    ///
    /// [RFC 6415 appendix A]: https://www.rfc-editor.org/rfc/rfc6415.html#appendix-A
    pub fn template<S: Into<String>>(mut self, template: S) -> Self {
        self.link.template = Some(template.into());
        self
    }

    /// Adds a single localized title to the link.
    ///
    /// RFC 7033 serializes titles as an object keyed by language tag, so repeated calls insert or
    /// replace entries in that object.
    ///
    /// [RFC 7033 section 4.4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4
    pub fn title<L: Into<String>, V: Into<String>>(mut self, language: L, value: V) -> Self {
        let title = Title::new(language, value);
        self.link
            .titles
            .get_or_insert_with(BTreeMap::new)
            .insert(title.language, title.value);
        self
    }

    /// Sets the complete language-keyed title object for the link.
    ///
    /// The argument can be any owned iterator of `(language, title)` pairs, including a moved
    /// `BTreeMap` or `HashMap`. Keys and values are converted into owned strings and stored as the
    /// JSON object described by [RFC 7033 section 4.4.4.4].
    ///
    /// [RFC 7033 section 4.4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4
    pub fn titles<I, L, V>(mut self, titles: I) -> Self
    where
        I: IntoIterator<Item = (L, V)>,
        L: Into<String>,
        V: Into<String>,
    {
        let titles = titles
            .into_iter()
            .map(|(language, value)| (language.into(), value.into()))
            .collect();
        self.link.titles = Some(titles);
        self
    }

    /// Adds a string-valued property to the link.
    ///
    /// The property identifier is validated as a [`JrdUri`]. The value serializes as a JSON string
    /// under that property key.
    ///
    /// [RFC 7033 section 4.4.4.5]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5
    pub fn property<K: AsRef<str>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.link
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(JrdUri::new(key), Some(value.into()));
        self
    }

    /// Adds a null-valued property to the link.
    ///
    /// This writes the property with a JSON `null` value. It is different from leaving the
    /// property out of the map.
    ///
    /// [RFC 7033 section 4.4.4.5]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5
    pub fn null_property<K: AsRef<str>>(mut self, key: K) -> Self {
        self.link
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(JrdUri::new(key), None);
        self
    }

    /// Sets the complete property object for the link.
    ///
    /// The argument can be any owned iterator of `(JrdUri, Option<String>)` pairs. Use `Some` for
    /// string-valued properties and `None` for JSON `null` values.
    ///
    /// [RFC 7033 section 4.4.4.5]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5
    pub fn properties<I>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = (JrdUri, Option<String>)>,
    {
        self.link.properties = Some(properties.into_iter().collect());
        self
    }

    /// Builds the link.
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
impl Debug for LinkBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LinkBuilder").field(&self.link).finish()
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
        if let Some(template) = &self.template {
            debug = debug.field("template", &template);
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
/// RFC 7033 serializes titles as a JSON object, not as a list of title objects. `Title` is a small
/// helper for builder-style construction where a caller wants to name one `(language, value)` pair
/// before it is inserted into the link's language-keyed map.
///
/// The language is stored as `String` because RFC 7033 points at language tags but does not require
/// WebFinger implementations to enforce a particular registry or normalization policy here.
///
/// See [RFC 7033 section 4.4.4.4].
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::{Link, Title};
///
/// let title = Title::new("en-us", "Carol's Profile");
/// let link = Link::builder("http://webfinger.net/rel/profile-page")
///     .title(title.language, title.value)
///     .build();
///
/// assert_eq!(
///     link.titles.unwrap().get("en-us").map(String::as_str),
///     Some("Carol's Profile"),
/// );
/// ```
///
/// [RFC 7033 section 4.4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.4
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Title {
    /// The language of the title.
    ///
    /// This can be any valid language tag as defined in [RFC
    /// 5646](https://www.rfc-editor.org/rfc/rfc5646.html) or the string `und` to indicate an
    /// undefined language.
    pub language: String,
    /// The title text for this language.
    pub value: String,
}

impl Title {
    /// Creates a title pair with the given language and value.
    pub fn new<L: Into<String>, V: Into<String>>(language: L, value: V) -> Self {
        Self {
            language: language.into(),
            value: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::hash::Hash;

    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;

    type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    fn assert_data_traits<T>()
    where
        T: Clone + Debug + Eq + Ord + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
    }

    fn assert_ordered_value_traits<T>()
    where
        T: Clone + Debug + Eq + Ord + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
    }

    fn assert_builder_traits<T>()
    where
        T: Clone + Debug + Eq + Ord + Hash + Send + Sync,
    {
    }

    /// Locks the expected trait surface for link data and builder values.
    ///
    /// Links are public JRD data, while builders often appear in tests before `.build()` is called;
    /// both should stay ergonomic to clone, compare, sort, hash, and debug.
    #[test]
    fn implements_applicable_common_traits() {
        assert_data_traits::<Link>();
        assert_ordered_value_traits::<Title>();
        assert_builder_traits::<LinkBuilder>();
    }

    /// Serializes localized titles as the RFC JRD language-object shape.
    ///
    /// RFC 7033 section 4.4.4.4 defines `titles` as an object keyed by language tag, not as an
    /// array of title entries.
    #[test]
    fn builder_serializes_titles_as_language_object() -> Result {
        let link = Link::builder("http://webfinger.net/rel/profile-page")
            .href("https://example.com/profile/carol")
            .title("en-us", "Carol's Profile")
            .build();

        let json = serde_json::to_value(link)?;

        assert_eq!(
            json,
            json!({
                "rel": "http://webfinger.net/rel/profile-page",
                "href": "https://example.com/profile/carol",
                "titles": {
                    "en-us": "Carol's Profile"
                }
            })
        );
        Ok(())
    }

    /// Serializes null link properties distinctly from absent properties.
    ///
    /// RFC 7033 allows property values to be `null`; callers use that to publish a known property
    /// with no value rather than omitting the property key entirely.
    #[test]
    fn builder_serializes_template() -> Result {
        let link = Link::builder("http://ostatus.org/schema/1.0/subscribe")
            .template("https://example.com/authorize_interaction?uri={uri}")
            .build();

        let json = serde_json::to_value(link)?;

        assert_eq!(
            json,
            json!({
                "rel": "http://ostatus.org/schema/1.0/subscribe",
                "template": "https://example.com/authorize_interaction?uri={uri}",
            })
        );
        Ok(())
    }

    #[test]
    fn deserializes_template() -> Result {
        let json = r#"
        {
          "rel": "copyright",
          "template": "http://example.com/copyright?id={uri}"
        }
        "#;

        let link: Link = serde_json::from_str(json)?;

        assert_eq!(link.rel.as_ref(), "copyright");
        assert_eq!(
            link.template.as_deref(),
            Some("http://example.com/copyright?id={uri}")
        );
        Ok(())
    }

    #[test]
    fn builder_serializes_null_properties() -> Result {
        const OLD_ROLE_PROPERTY: &str = "https://example.com/ns/old-role";
        const ROLE_PROPERTY: &str = "https://example.com/ns/role";

        let link = Link::builder("author")
            .property(ROLE_PROPERTY, "editor")
            .null_property(OLD_ROLE_PROPERTY)
            .build();

        let json = serde_json::to_value(link)?;

        assert_eq!(
            json,
            json!({
                "rel": "author",
                "properties": {
                    "https://example.com/ns/role": "editor",
                    "https://example.com/ns/old-role": null
                }
            })
        );
        Ok(())
    }

    /// Keeps the direct constructor limited to the required relation member.
    ///
    /// Optional JRD link members should only appear when callers choose them through the builder or
    /// direct struct construction, and the compact debug output should not print absent fields.
    #[test]
    fn new_sets_only_relation() {
        let link = Link::new(Rel::new("avatar"));

        assert_eq!(
            link,
            Link {
                rel: Rel::new("avatar"),
                r#type: None,
                href: None,
                template: None,
                titles: None,
                properties: None,
            },
        );
    }

    /// Omits absent optional fields from minimal link debug output.
    #[test]
    fn debug_omits_absent_optional_fields() {
        let link = Link::new(Rel::new("avatar"));

        assert_eq!(format!("{link:?}"), r#"Link { rel: Rel("avatar") }"#);
    }

    /// Covers the replacement-style builder methods that set whole optional objects at once.
    ///
    /// The one-at-a-time methods already have serialization coverage; this guards the bulk title
    /// and property setters used when callers have pre-collected maps.
    #[test]
    fn builder_serializes_complete_title_and_property_maps() -> Result {
        const LEGACY_PROPERTY: &str = "https://example.com/ns/legacy";
        const ROLE_PROPERTY: &str = "https://example.com/ns/role";

        let link = Link::builder("author")
            .r#type("text/html")
            .href("https://example.com/people/carol")
            .titles([("en-us", "Carol"), ("fr", "Caroline")])
            .properties([
                (JrdUri::new(ROLE_PROPERTY), Some("editor".to_string())),
                (JrdUri::new(LEGACY_PROPERTY), None),
            ])
            .build();

        assert_eq!(
            serde_json::to_value(link)?,
            json!({
                "rel": "author",
                "type": "text/html",
                "href": "https://example.com/people/carol",
                "titles": {
                    "en-us": "Carol",
                    "fr": "Caroline"
                },
                "properties": {
                    "https://example.com/ns/legacy": null,
                    "https://example.com/ns/role": "editor"
                }
            })
        );
        Ok(())
    }

    /// Includes optional fields in debug output only when they are present.
    ///
    /// The custom debug formatter is meant to keep minimal links compact while still exposing
    /// populated JRD members during test failures and logs.
    #[test]
    fn debug_includes_present_optional_fields() {
        const ROLE_PROPERTY: &str = "https://example.com/ns/role";

        let link = Link::builder("author")
            .r#type("text/html")
            .href("https://example.com/people/carol")
            .title("en-us", "Carol")
            .property(ROLE_PROPERTY, "editor")
            .build();

        assert_eq!(
            format!("{link:?}"),
            r#"Link { rel: Rel("author"), type: "text/html", href: JrdUri("https://example.com/people/carol"), titles: {"en-us": "Carol"}, properties: {JrdUri("https://example.com/ns/role"): Some("editor")} }"#
        );
    }

    /// Keeps builder debug output recognizable while delegating field detail to `Link`.
    ///
    /// Builder values can appear in assertion failures before `.build()` is called, so their debug
    /// shape should remain intentional rather than falling back to private field names.
    #[test]
    fn builder_debug_wraps_inner_link() {
        let builder = Link::builder("author");

        assert_eq!(
            format!("{builder:?}"),
            r#"LinkBuilder(Link { rel: Rel("author") })"#
        );
    }

    /// Replaces a prior localized title with the same language key.
    ///
    /// JRD titles are an object keyed by language tag, so adding the same language twice should
    /// behave like map insertion rather than producing a duplicate title entry.
    #[test]
    fn title_replaces_existing_language() {
        let link = Link::builder("author")
            .title("en-us", "Carol")
            .title("en-us", "Carol Smith")
            .build();

        assert_eq!(
            link.titles
                .as_ref()
                .and_then(|titles| titles.get("en-us"))
                .map(String::as_str),
            Some("Carol Smith")
        );
    }

    /// Rejects array-shaped localized titles.
    ///
    /// This guards the RFC JRD object shape for `titles` rather than accepting a more generic
    /// language/value list shape.
    #[test]
    fn deserialization_rejects_title_array_shape() {
        let json = r#"
        {
          "rel": "author",
          "titles": [
            {
              "language": "en-us",
              "value": "Carol"
            }
          ]
        }
        "#;

        let error = serde_json::from_str::<Link>(json).expect_err("title array");

        assert!(error.to_string().contains("invalid type"));
    }

    /// Rejects relative URI property identifiers during link deserialization.
    ///
    /// Link property names are JRD URI strings, so accepting relative keys would create an invalid
    /// typed `Link` from inbound JSON.
    #[test]
    fn deserialization_rejects_relative_property_identifiers() {
        let json = r#"
        {
          "rel": "author",
          "properties": {
            "/ns/role": "editor"
          }
        }
        "#;

        let error = serde_json::from_str::<Link>(json).expect_err("relative property identifier");

        assert!(error.to_string().contains("invalid JRD URI"));
    }
}
