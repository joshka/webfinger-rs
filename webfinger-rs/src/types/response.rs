use std::collections::BTreeMap;
use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::Error;
use crate::{JrdUri, Link};

/// A WebFinger response.
///
/// This is the JSON Resource Descriptor (JRD) returned by a WebFinger server. The Rust fields map
/// directly to the top-level members from RFC 7033:
///
/// - [`subject`](Self::subject) is required and uses [`JrdUri`] because the RFC defines it as the
///   URI of the resource described by the JRD.
/// - [`aliases`](Self::aliases) is an optional list of URI strings, also represented as
///   [`JrdUri`].
/// - [`properties`](Self::properties) is an optional object with URI property identifiers and
///   string-or-null values.
/// - [`links`](Self::links) is the JRD link array. Missing `links` deserializes as an empty
///   vector.
///
/// The response serializes to the RFC JSON shape. It uses typed wrappers for URI-valued and
/// relation-valued fields while keeping builder methods string-friendly for application code.
///
/// See [RFC 7033 section 4.4].
///
/// # Examples
///
/// Constructing a response with builders keeps common server code concise:
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
/// JSON `null` property values are represented with `null_property` on the builder:
///
/// ```rust
/// use webfinger_rs::{Link, WebFingerResponse};
///
/// let response = WebFingerResponse::builder("acct:carol@example.com")
///     .property("https://example.com/ns/role", "developer")
///     .null_property("https://example.com/ns/previous-role")
///     .link(
///         Link::builder("author")
///             .href("https://example.com/people/carol")
///             .null_property("https://example.com/ns/legacy-page"),
///     )
///     .build();
///
/// let json = serde_json::to_value(response)?;
/// assert_eq!(
///     json["properties"]["https://example.com/ns/previous-role"],
///     serde_json::Value::Null,
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// `Response` can be used as a response in Axum handlers as it implements
/// [`axum::response::IntoResponse`].
///
/// [`axum::response::IntoResponse`]:
///     https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html
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
///
/// [RFC 7033 section 4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// The subject of the response.
    ///
    /// This is the URI of the resource that the JRD describes. RFC 7033 makes it required when a
    /// response is returned, so the Rust field is not optional.
    ///
    /// [`JrdUri`] is used instead of `String` so relative references are rejected during
    /// deserialization and builder construction.
    ///
    /// See [RFC 7033 section 4.4.1].
    ///
    /// [RFC 7033 section 4.4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.1
    pub subject: JrdUri,

    /// The aliases of the response.
    ///
    /// Aliases are additional URI strings for the same subject. The field is optional because the
    /// JSON member may be absent. Each value is a [`JrdUri`] for the same reason as
    /// [`Response::subject`].
    ///
    /// See [RFC 7033 section 4.4.2].
    ///
    /// [RFC 7033 section 4.4.2]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.2
    pub aliases: Option<Vec<JrdUri>>,

    /// The properties of the response.
    ///
    /// JRD properties are a JSON object whose names are URI strings. Values may be strings or JSON
    /// `null`, so the Rust value type is `Option<String>`. `None` serializes as a property value of
    /// `null`; it does not omit the property from the map.
    ///
    /// A `BTreeMap` is used for deterministic ordering and to support the standard ordering and
    /// hashing traits on `Response`.
    ///
    /// See [RFC 7033 section 4.4.3].
    ///
    /// [RFC 7033 section 4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3
    pub properties: Option<BTreeMap<JrdUri, Option<String>>>,

    /// The links of the response.
    ///
    /// This is the JRD `links` array. A missing JSON member deserializes to an empty vector so code
    /// can iterate links without handling a separate absent state.
    ///
    /// See [RFC 7033 section 4.4.4] and [`Link`].
    ///
    /// [RFC 7033 section 4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Response {
    /// Creates a response with the given subject and no optional JRD members.
    ///
    /// This constructor is intended for application-controlled subject strings. It validates the
    /// subject as a [`JrdUri`] and panics if the string is not an absolute URI. Use
    /// [`Response::try_builder`] when the subject comes from external input.
    pub fn new<S: AsRef<str>>(subject: S) -> Self {
        Self {
            subject: JrdUri::new(subject),
            aliases: None,
            properties: None,
            links: Vec::new(),
        }
    }

    /// Creates a [`Builder`] with the given subject.
    ///
    /// The builder accepts strings at the API boundary and stores typed JRD values internally. This
    /// keeps straightforward server responses concise while still producing the RFC-shaped JSON
    /// object.
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
    pub fn builder<S: AsRef<str>>(subject: S) -> Builder {
        Builder::new(subject)
    }

    /// Tries to create a new [`Builder`] with the given subject.
    ///
    /// Use this when the subject string has not already been validated by application logic. The
    /// returned builder has the same methods as [`Response::builder`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use webfinger_rs::WebFingerResponse;
    ///
    /// assert!(WebFingerResponse::try_builder("acct:carol@example.com").is_ok());
    /// assert!(WebFingerResponse::try_builder("/users/carol").is_err());
    /// ```
    pub fn try_builder<S: AsRef<str>>(subject: S) -> Result<Builder, Error> {
        Ok(Builder::new(JrdUri::try_new(subject)?))
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string_pretty(self).unwrap())
    }
}

/// A builder for a WebFinger response.
///
/// `Builder` constructs a [`Response`] using the JRD member names from RFC 7033. It is the
/// preferred API for ordinary server responses because it accepts string-like values and converts
/// them to [`JrdUri`] or [`Link`] where the stored response type is stricter than JSON text.
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::{Link, WebFingerResponse};
///
/// let response = WebFingerResponse::builder("acct:carol@example.com")
///     .alias("https://example.com/users/carol")
///     .property("https://example.com/ns/display-name", "Carol")
///     .link(Link::builder("avatar").href("https://example.com/avatar/carol.png"))
///     .build();
///
/// assert_eq!(response.subject.as_ref(), "acct:carol@example.com");
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Builder {
    response: Response,
}

impl Builder {
    /// Creates a response builder with the given subject.
    ///
    /// The subject is validated immediately as a [`JrdUri`].
    pub fn new<S: AsRef<str>>(subject: S) -> Self {
        Self {
            response: Response::new(subject),
        }
    }

    /// Adds an alias URI to the response.
    ///
    /// The value is validated as a [`JrdUri`] and serialized in the `aliases` array from
    /// [RFC 7033 section 4.4.2].
    ///
    /// [RFC 7033 section 4.4.2]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.2
    pub fn alias<S: AsRef<str>>(mut self, alias: S) -> Self {
        self.response
            .aliases
            .get_or_insert_with(Vec::new)
            .push(JrdUri::new(alias));
        self
    }

    /// Adds a string-valued property to the response.
    ///
    /// The key is validated as a [`JrdUri`]. The value serializes as a JSON string under that
    /// property identifier.
    ///
    /// [RFC 7033 section 4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3
    pub fn property<K: AsRef<str>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.response
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(JrdUri::new(key), Some(value.into()));
        self
    }

    /// Adds a null-valued property to the response.
    ///
    /// This writes the property with a JSON `null` value. It is different from leaving the
    /// property out of the JRD object.
    ///
    /// [RFC 7033 section 4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3
    pub fn null_property<K: AsRef<str>>(mut self, key: K) -> Self {
        self.response
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(JrdUri::new(key), None);
        self
    }

    /// Adds a link to the response.
    ///
    /// If the link is constructed with a builder, it is not necessary to call the `build` method on
    /// the link as the builder implements `From<LinkBuilder> for Link`.
    ///
    /// This appends to the `links` array from [RFC 7033 section 4.4.4].
    ///
    /// [RFC 7033 section 4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4
    pub fn link<L: Into<Link>>(mut self, link: L) -> Self {
        self.response.links.push(link.into());
        self
    }

    /// Sets the complete link array for the response.
    ///
    /// Use this when links are already collected. Use [`Builder::link`] when appending links one at
    /// a time.
    ///
    /// [RFC 7033 section 4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4
    pub fn links(mut self, links: Vec<Link>) -> Self {
        self.response.links = links;
        self
    }

    /// Builds the response.
    pub fn build(self) -> Response {
        self.response
    }
}

/// Custom debug implementation to avoid printing `None` fields
impl Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Builder").field(&self.response).finish()
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

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display};
    use std::hash::Hash;

    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::Rel;

    type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    fn assert_data_traits<T>()
    where
        T: Clone
            + Debug
            + Display
            + Eq
            + Ord
            + Hash
            + Send
            + Sync
            + Serialize
            + for<'de> Deserialize<'de>,
    {
    }

    fn assert_builder_traits<T>()
    where
        T: Clone + Debug + Eq + Ord + Hash + Send + Sync,
    {
    }

    /// Locks the expected trait surface for response data and builder values.
    ///
    /// Responses are public JRD data used in maps, test assertions, serialization, and debug logs;
    /// builders should also remain easy to clone and compare while constructing fixtures.
    #[test]
    fn implements_applicable_common_traits() {
        assert_data_traits::<Response>();
        assert_builder_traits::<Builder>();
    }

    /// Deserializes a representative RFC-shaped JRD document.
    ///
    /// This covers the nested object shapes that are easy to accidentally model as simpler maps or
    /// lists: response properties, link titles, link properties, and explicit `null` property
    /// values.
    #[test]
    fn deserializes_rfc_shaped_jrd_with_null_properties_and_title_object() -> Result {
        let json = r#"
        {
          "subject": "http://blog.example.com/article/id/314",
          "aliases": [
            "http://blog.example.com/cool_new_thing",
            "http://blog.example.com/steve/article/7"
          ],
          "properties": {
            "http://blgx.example.net/ns/version": "1.3",
            "http://blgx.example.net/ns/ext": null
          },
          "links": [
            {
              "rel": "author",
              "href": "http://blog.example.com/author/steve",
              "titles": {
                "en-us": "The Magical World of Steve",
                "fr": "Le Monde Magique de Steve"
              },
              "properties": {
                "http://example.com/role": "editor",
                "http://example.com/old-role": null
              }
            }
          ]
        }
        "#;

        let response = serde_json::from_str::<Response>(json)?;
        let properties = response.properties.as_ref().expect("properties");
        let links = &response.links;
        let link = links.first().expect("link");
        let titles = link.titles.as_ref().expect("titles");
        let link_properties = link.properties.as_ref().expect("link properties");

        assert_eq!(
            response.subject.as_ref(),
            "http://blog.example.com/article/id/314"
        );
        assert_eq!(
            response.aliases.as_ref().expect("aliases")[0].as_ref(),
            "http://blog.example.com/cool_new_thing"
        );
        assert_eq!(
            properties
                .get(&JrdUri::new("http://blgx.example.net/ns/version"))
                .expect("version")
                .as_deref(),
            Some("1.3")
        );
        assert_eq!(
            properties.get(&JrdUri::new("http://blgx.example.net/ns/ext")),
            Some(&None)
        );
        assert_eq!(link.rel, Rel::new("author"));
        assert_eq!(
            link.href.as_ref().expect("href").as_ref(),
            "http://blog.example.com/author/steve"
        );
        assert_eq!(
            titles.get("en-us").map(String::as_str),
            Some("The Magical World of Steve")
        );
        assert_eq!(
            link_properties.get(&JrdUri::new("http://example.com/old-role")),
            Some(&None)
        );
        Ok(())
    }

    /// Serializes builder output into the RFC JRD object shape.
    ///
    /// This is the inverse of the deserialization coverage above: builders should produce the same
    /// aliases, properties, link titles, link properties, and `null` values that inbound JRDs use.
    #[test]
    fn serializes_builder_output_as_rfc_shaped_jrd() -> Result {
        const LEGACY_LINK_PROPERTY: &str = "https://example.com/ns/legacy";
        const OLD_ROLE_PROPERTY: &str = "https://example.com/ns/old-role";
        const ROLE_PROPERTY: &str = "https://example.com/ns/role";
        const VERIFIED_PROPERTY: &str = "https://example.com/ns/verified";

        let response = Response::builder("acct:carol@example.com")
            .alias("https://example.com/profile/carol")
            .property(ROLE_PROPERTY, "developer")
            .null_property(OLD_ROLE_PROPERTY)
            .link(
                Link::builder("http://webfinger.net/rel/profile-page")
                    .href("https://example.com/profile/carol")
                    .title("en-us", "Carol's Profile")
                    .property(VERIFIED_PROPERTY, "true")
                    .null_property(LEGACY_LINK_PROPERTY),
            )
            .build();

        let json = serde_json::to_value(response)?;

        assert_eq!(
            json,
            json!({
                "subject": "acct:carol@example.com",
                "aliases": ["https://example.com/profile/carol"],
                "properties": {
                    "https://example.com/ns/role": "developer",
                    "https://example.com/ns/old-role": null
                },
                "links": [
                    {
                        "rel": "http://webfinger.net/rel/profile-page",
                        "href": "https://example.com/profile/carol",
                        "titles": {
                            "en-us": "Carol's Profile"
                        },
                        "properties": {
                            "https://example.com/ns/verified": "true",
                            "https://example.com/ns/legacy": null
                        }
                    }
                ]
            })
        );
        Ok(())
    }

    /// Keeps the direct constructor focused on the required JRD subject.
    ///
    /// Optional members are intentionally absent until requested, while `links` remains an empty
    /// array because response code can iterate links without handling a missing field.
    #[test]
    fn new_sets_required_subject_only() {
        let response = Response::new("acct:carol@example.com");

        assert_eq!(
            response,
            Response {
                subject: JrdUri::new("acct:carol@example.com"),
                aliases: None,
                properties: None,
                links: Vec::new(),
            },
        );
    }

    /// Serializes minimal responses with the required subject and an empty link array.
    #[test]
    fn new_serializes_required_subject_and_empty_links() -> Result {
        let response = Response::new("acct:carol@example.com");

        assert_eq!(
            serde_json::to_value(&response)?,
            json!({
                "subject": "acct:carol@example.com",
                "links": []
            }),
        );
        Ok(())
    }

    /// Pretty-prints responses through the same JSON shape exposed to callers.
    #[test]
    fn display_pretty_prints_response_json() {
        let response = Response::new("acct:carol@example.com");

        assert_eq!(
            response.to_string(),
            "{\n  \"subject\": \"acct:carol@example.com\",\n  \"links\": []\n}"
        );
    }

    /// Omits absent optional fields from minimal response debug output.
    #[test]
    fn debug_omits_absent_optional_fields() {
        let response = Response::new("acct:carol@example.com");

        assert_eq!(
            format!("{response:?}"),
            r#"Response { subject: JrdUri("acct:carol@example.com"), links: [] }"#
        );
    }

    /// Rejects invalid subjects through the fallible builder path.
    ///
    /// `Response::builder` is intentionally convenient and panicking for trusted strings; this
    /// fallible constructor is the API external input should use instead.
    #[test]
    fn try_builder_rejects_relative_subject() {
        let error = Response::try_builder("/users/carol").expect_err("relative subject");

        assert!(matches!(error, Error::InvalidJrdUri(uri) if uri == "/users/carol"));
    }

    /// Replaces the complete link array when callers already have a collected list.
    ///
    /// This distinguishes `links(...)` from `link(...)`, which appends one item at a time.
    #[test]
    fn links_replaces_existing_link_array() {
        let response = Response::builder("acct:carol@example.com")
            .link(Link::builder("author"))
            .links(vec![Link::builder("avatar").build()])
            .build();

        assert_eq!(response.links, vec![Link::builder("avatar").build()]);
    }

    /// Includes optional response objects in debug output only when present.
    ///
    /// The custom debug formatter keeps minimal responses short, but populated responses should
    /// still expose aliases and properties when a test or log captures the value.
    #[test]
    fn debug_includes_present_optional_fields() {
        const ROLE_PROPERTY: &str = "https://example.com/ns/role";

        let response = Response::builder("acct:carol@example.com")
            .alias("https://example.com/people/carol")
            .property(ROLE_PROPERTY, "admin")
            .build();

        assert_eq!(
            format!("{response:?}"),
            r#"Response { subject: JrdUri("acct:carol@example.com"), aliases: [JrdUri("https://example.com/people/carol")], properties: {JrdUri("https://example.com/ns/role"): Some("admin")}, links: [] }"#
        );
    }

    /// Keeps builder debug output recognizable while delegating field detail to `Response`.
    ///
    /// Builder values can appear in assertion failures before `.build()` is called, so their debug
    /// shape should remain intentional rather than exposing private field names.
    #[test]
    fn builder_debug_wraps_inner_response() {
        let builder = Response::builder("acct:carol@example.com");

        assert_eq!(
            format!("{builder:?}"),
            r#"Builder(Response { subject: JrdUri("acct:carol@example.com"), links: [] })"#
        );
    }

    /// Rejects relative URI-valued fields in inbound JRD documents.
    ///
    /// Link `href` values are JRD URI strings, so relative references should fail during
    /// deserialization instead of entering typed response data.
    #[test]
    fn rejects_relative_jrd_uris() {
        let json =
            r#"{"subject":"acct:carol@example.com","links":[{"rel":"author","href":"/carol"}]}"#;

        let error = serde_json::from_str::<Response>(json).expect_err("relative href");

        assert!(error.to_string().contains("invalid JRD URI"));
    }

    /// Rejects invalid relation values inside inbound JRD links.
    ///
    /// Response deserialization should enforce the same `Rel` boundary as builders and request
    /// filters.
    #[test]
    fn rejects_empty_relation_types() {
        let json = r#"{"subject":"acct:carol@example.com","links":[{"rel":""}]}"#;

        let error = serde_json::from_str::<Response>(json).expect_err("empty rel");

        assert!(error.to_string().contains("invalid relation type"));
    }

    /// Defaults missing `links` to an empty collection.
    ///
    /// RFC-shaped JRDs can omit optional members; typed response code should still be able to
    /// iterate links without handling `None`.
    #[test]
    fn deserializes_jrd_without_links() -> Result {
        let json = r#"{"subject":"acct:carol@example.com"}"#;

        let response = serde_json::from_str::<Response>(json)?;

        assert!(response.links.is_empty());
        Ok(())
    }
}
