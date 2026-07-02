use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;
use crate::types::jrd_uri::is_absolute_uri;

/// Link relation type.
///
/// WebFinger relation types are either absolute URI strings or IANA-registered relation type
/// names. RFC 7033 requires each `rel` member to contain exactly one relation type, so this type
/// rejects empty strings, relative URI references, and strings that try to carry multiple
/// relation types.
///
/// `Rel` serializes as a JSON string and implements [`AsRef<str>`] for comparison or lookup
/// without allocating. Builder methods accept strings for common use, but store this validated type
/// so deserialized and programmatically built links use the same representation.
///
/// Registered relation type names use the `reg-rel-type` syntax from [RFC 5988 section 5.3].
/// URI-valued relation types are validated as absolute URI strings under RFC 3986, including the
/// [section 2.1] percent-encoding rule.
///
/// See [RFC 7033 section 4.4.4.1].
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::Rel;
///
/// let registered = Rel::try_new("author")?;
/// let uri = Rel::try_new("http://webfinger.net/rel/profile-page")?;
///
/// assert_eq!(registered.as_ref(), "author");
/// assert_eq!(uri.as_ref(), "http://webfinger.net/rel/profile-page");
/// # Ok::<(), webfinger_rs::Error>(())
/// ```
///
/// Multiple relation types belong in multiple request `rel` parameters or multiple links, not in
/// one `Rel` value:
///
/// ```rust
/// use webfinger_rs::Rel;
///
/// assert!(Rel::try_new("author avatar").is_err());
/// ```
///
/// [RFC 7033 section 4.4.4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.1
/// [RFC 5988 section 5.3]: https://www.rfc-editor.org/rfc/rfc5988.html#section-5.3
/// [section 2.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rel(String);

impl Rel {
    /// Creates a link relation type.
    ///
    /// This constructor is convenient for application-controlled relation strings. Use
    /// [`Rel::try_new`] when parsing external input.
    ///
    /// # Panics
    ///
    /// Panics if `rel` is not a URI relation type or registered relation type name. Use
    /// [`Rel::try_new`] when handling untrusted input.
    pub fn new<S: AsRef<str>>(rel: S) -> Self {
        Self::try_new(rel).expect("invalid WebFinger link relation type")
    }

    /// Tries to create a link relation type.
    ///
    /// URI relation types must be absolute URI strings. Registered relation type names follow the
    /// `reg-rel-type` syntax from RFC 5988: a lowercase ASCII letter followed by lowercase ASCII
    /// letters, digits, `.`, or `-`.
    pub fn try_new<S: AsRef<str>>(rel: S) -> Result<Self, Error> {
        let rel = rel.as_ref();
        if is_absolute_uri(rel) || is_registered_relation_type(rel) {
            Ok(Self(rel.to_string()))
        } else {
            Err(Error::InvalidRel(rel.to_string()))
        }
    }
}

impl fmt::Display for Rel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Rel {
    type Err = Error;

    fn from_str(rel: &str) -> Result<Self, Self::Err> {
        Self::try_new(rel)
    }
}

impl TryFrom<&str> for Rel {
    type Error = Error;

    fn try_from(rel: &str) -> Result<Self, Self::Error> {
        Self::try_new(rel)
    }
}

impl TryFrom<String> for Rel {
    type Error = Error;

    fn try_from(rel: String) -> Result<Self, Self::Error> {
        Self::try_new(rel)
    }
}

impl From<Rel> for String {
    fn from(rel: Rel) -> Self {
        rel.0
    }
}

impl AsRef<str> for Rel {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Rel {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Serialize for Rel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Rel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RelVisitor)
    }
}

struct RelVisitor;

impl Visitor<'_> for RelVisitor {
    type Value = Rel;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a URI relation type or registered relation type")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Rel::try_new(value).map_err(E::custom)
    }
}

fn is_registered_relation_type(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fmt::{Debug, Display};
    use std::hash::Hash;

    use serde::{Deserialize, Serialize};

    use super::*;

    fn assert_common_traits<T>()
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

    #[test]
    fn implements_applicable_common_traits() {
        assert_common_traits::<Rel>();
    }

    #[test]
    fn accepts_uri_relation_types() {
        let rel = Rel::try_new("http://webfinger.net/rel/profile-page").unwrap();

        assert_eq!(rel.as_ref(), "http://webfinger.net/rel/profile-page");
    }

    #[test]
    fn accepts_registered_relation_types() {
        let rel = Rel::try_new("author").unwrap();

        assert_eq!(rel.as_ref(), "author");
    }

    #[test]
    fn try_from_parses_valid_relation_types() {
        let rel = Rel::try_from("author").unwrap();

        assert_eq!(rel.as_ref(), "author");
    }

    #[test]
    fn converts_back_into_owned_string() {
        let rel = Rel::new("author");

        assert_eq!(String::from(rel), "author");
    }

    #[test]
    fn supports_borrowed_string_set_lookup() {
        let mut values = BTreeSet::new();
        values.insert(Rel::new("author"));

        assert!(values.contains("author"));
    }

    #[test]
    fn orders_by_relation_string() {
        let first = Rel::new("author");
        let second = Rel::new("http://webfinger.net/rel/profile-page");

        assert!(first < second);
    }

    #[test]
    fn rejects_empty_relation_types() {
        let error = Rel::try_new("").expect_err("empty relation type");

        assert!(error.to_string().contains("invalid relation type"));
    }

    #[test]
    fn rejects_multiple_relation_types_in_one_value() {
        let error = Rel::try_new("author avatar").expect_err("multiple relation types");

        assert!(error.to_string().contains("invalid relation type"));
    }

    #[test]
    fn rejects_relative_uri_relation_types() {
        let error = Rel::try_new("/rel/profile-page").expect_err("relative URI relation type");

        assert!(error.to_string().contains("invalid relation type"));
    }

    #[test]
    fn rejects_uri_relation_types_with_malformed_percent_escapes() {
        let error = Rel::try_new("http://example.com/a%GG").expect_err("malformed percent escape");

        assert!(error.to_string().contains("invalid relation type"));
    }

    #[test]
    fn deserialization_rejects_invalid_relation_types() {
        let error = serde_json::from_str::<Rel>(r#""""#).expect_err("empty relation type");

        assert!(error.to_string().contains("invalid relation type"));
    }
}
