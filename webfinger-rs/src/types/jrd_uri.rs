use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

/// A URI string used in a WebFinger JRD response.
///
/// RFC 7033 defines the JRD `subject`, `aliases`, link `href`, and property identifiers as URI
/// strings. This type keeps those fields distinct from free-form text and rejects relative URI
/// references when values are parsed from JSON or constructed with [`JrdUri::try_new`].
///
/// `JrdUri` serializes as a JSON string, implements [`AsRef<str>`] for borrowed access, and can be
/// converted back into a [`String`] when a caller needs owned text. Builder methods accept strings
/// for ergonomics, then store validated `JrdUri` values internally.
///
/// See [RFC 7033 section 4.4.1] for `subject`, [section 4.4.2] for `aliases`,
/// [section 4.4.3] for response properties, [section 4.4.4.3] for link `href`, and
/// [section 4.4.4.5] for link properties. Those sections rely on URI syntax from RFC 3986,
/// including [section 2.1] percent escapes and [section 4.3] absolute URIs.
///
/// # Examples
///
/// Fallible construction is useful when accepting input from users, configuration, or another
/// service:
///
/// ```rust
/// use webfinger_rs::JrdUri;
///
/// let subject = JrdUri::try_new("acct:carol@example.com")?;
/// assert_eq!(subject.as_ref(), "acct:carol@example.com");
/// # Ok::<(), webfinger_rs::Error>(())
/// ```
///
/// Relative references are rejected:
///
/// ```rust
/// use webfinger_rs::JrdUri;
///
/// assert!(JrdUri::try_new("/users/carol").is_err());
/// ```
///
/// [RFC 7033 section 4.4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.1
/// [section 4.4.2]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.2
/// [section 4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.3
/// [section 4.4.4.3]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.3
/// [section 4.4.4.5]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4.5
/// [section 2.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1
/// [section 4.3]: https://www.rfc-editor.org/rfc/rfc3986.html#section-4.3
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JrdUri(String);

impl JrdUri {
    /// Creates a JRD URI.
    ///
    /// This constructor is intended for URI strings controlled by the application, such as
    /// constants and values already validated by routing or configuration code. Use
    /// [`JrdUri::try_new`] for fallible construction from external input.
    ///
    /// # Panics
    ///
    /// Panics if `uri` is not an absolute URI string. Use [`JrdUri::try_new`] when handling
    /// untrusted input.
    pub fn new<S: AsRef<str>>(uri: S) -> Self {
        Self::try_new(uri).expect("invalid WebFinger JRD URI")
    }

    /// Tries to create a JRD URI from an absolute URI string.
    ///
    /// The value is stored without normalization. This preserves the string that will be serialized
    /// into the JRD while still checking that callers did not pass relative references or ordinary
    /// labels by mistake.
    pub fn try_new<S: AsRef<str>>(uri: S) -> Result<Self, Error> {
        let uri = uri.as_ref();
        if is_absolute_uri(uri) {
            Ok(Self(uri.to_string()))
        } else {
            Err(Error::InvalidJrdUri(uri.to_string()))
        }
    }
}

impl fmt::Display for JrdUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for JrdUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JrdUri").field(&self.0).finish()
    }
}

impl FromStr for JrdUri {
    type Err = Error;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        Self::try_new(uri)
    }
}

impl TryFrom<&str> for JrdUri {
    type Error = Error;

    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        Self::try_new(uri)
    }
}

impl TryFrom<String> for JrdUri {
    type Error = Error;

    fn try_from(uri: String) -> Result<Self, Self::Error> {
        Self::try_new(uri)
    }
}

impl From<JrdUri> for String {
    fn from(uri: JrdUri) -> Self {
        uri.0
    }
}

impl AsRef<str> for JrdUri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for JrdUri {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Serialize for JrdUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for JrdUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(JrdUriVisitor)
    }
}

struct JrdUriVisitor;

impl Visitor<'_> for JrdUriVisitor {
    type Value = JrdUri;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an absolute URI string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        JrdUri::try_new(value).map_err(E::custom)
    }
}

/// Returns whether `value` is an absolute URI string under the crate's JRD URI policy.
///
/// RFC 7033's JRD members call these values URI strings, not arbitrary text. This helper enforces
/// the pieces the crate relies on before storing a [`JrdUri`] or accepting a URI-valued [`Rel`]:
/// a valid RFC 3986 scheme, syntactically valid percent escapes, and successful parsing by
/// [`http::Uri`]. The explicit percent-escape check is necessary because `http::Uri` accepts
/// malformed escapes such as `%GG`, while RFC 3986 section 2.1 constrains percent encoding to `%`
/// followed by two hexadecimal digits.
///
/// [`Rel`]: crate::Rel
///
/// See [RFC 3986 section 2.1] for percent encoding, [section 3.1] for scheme syntax, and
/// [section 4.3] for absolute URI syntax.
///
/// [RFC 3986 section 2.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1
/// [section 3.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-3.1
/// [section 4.3]: https://www.rfc-editor.org/rfc/rfc3986.html#section-4.3
pub(crate) fn is_absolute_uri(value: &str) -> bool {
    let Some((scheme, _rest)) = value.split_once(':') else {
        return false;
    };
    let has_scheme = is_uri_scheme(scheme);
    let has_valid_percent_encoding = has_valid_percent_escapes(value);
    let parses_as_uri = value.parse::<http::Uri>().is_ok();

    has_scheme && has_valid_percent_encoding && parses_as_uri
}

/// Returns whether every `%` starts a complete RFC 3986 percent escape.
///
/// RFC 3986 section 2.1 defines `pct-encoded` as `%` followed by exactly two hexadecimal digits.
/// Some URI parsers preserve malformed escapes as ordinary path text, so URI-valued WebFinger
/// fields need this check before accepting user or JSON input as validated URI text.
///
/// See [RFC 3986 section 2.1].
///
/// [RFC 3986 section 2.1]: https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1
fn has_valid_percent_escapes(value: &str) -> bool {
    let mut bytes = value.as_bytes().iter();
    while let Some(byte) = bytes.next() {
        if *byte != b'%' {
            continue;
        }
        let Some(high) = bytes.next() else {
            return false;
        };
        let Some(low) = bytes.next() else {
            return false;
        };
        if !high.is_ascii_hexdigit() || !low.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_uri_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
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
        assert_common_traits::<JrdUri>();
    }

    #[test]
    fn accepts_absolute_uri_strings() {
        let uri = JrdUri::try_new("acct:carol@example.com").unwrap();

        assert_eq!(uri.as_ref(), "acct:carol@example.com");
    }

    #[test]
    fn try_from_parses_valid_uri_strings() {
        let uri = JrdUri::try_from("acct:carol@example.com").unwrap();

        assert_eq!(uri.as_ref(), "acct:carol@example.com");
    }

    #[test]
    fn converts_back_into_owned_string() {
        let uri = JrdUri::new("acct:carol@example.com");

        assert_eq!(String::from(uri), "acct:carol@example.com");
    }

    #[test]
    fn supports_borrowed_string_map_lookup() {
        let mut values = BTreeMap::new();
        values.insert(JrdUri::new("acct:carol@example.com"), "Carol");

        assert_eq!(values.get("acct:carol@example.com"), Some(&"Carol"));
    }

    #[test]
    fn orders_by_uri_string() {
        let first = JrdUri::new("acct:alice@example.com");
        let second = JrdUri::new("acct:carol@example.com");

        assert!(first < second);
    }

    #[test]
    fn rejects_relative_uri_references() {
        let error = JrdUri::try_new("/profile/carol").expect_err("relative URI");

        assert!(error.to_string().contains("invalid JRD URI"));
    }

    #[test]
    fn rejects_non_uri_strings() {
        let error = JrdUri::try_new("carol@example.com").expect_err("non-URI string");

        assert!(error.to_string().contains("invalid JRD URI"));
    }

    #[test]
    fn rejects_malformed_percent_escapes() {
        for uri in ["https://example.org/a%GG", "acct:carol%GG@example.com"] {
            let error = JrdUri::try_new(uri).expect_err("malformed percent escape");

            assert!(
                error.to_string().contains("invalid JRD URI"),
                "expected invalid JRD URI error for {uri:?}, got {error:?}",
            );
        }
    }

    #[test]
    fn deserialization_rejects_relative_uri_references() {
        let error =
            serde_json::from_str::<JrdUri>(r#""/profile/carol""#).expect_err("relative URI");

        assert!(error.to_string().contains("invalid JRD URI"));
    }
}
