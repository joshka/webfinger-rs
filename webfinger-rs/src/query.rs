//! Internal parsing for incoming WebFinger query strings.

use std::str::FromStr;

use percent_encoding::percent_decode_str;
use thiserror::Error;

use crate::{Resource, ResourceError};

/// The query parameters for a WebFinger request.
///
/// `resource` is required exactly once by RFC 7033 sections 4.1 and 4.2 and must be an absolute
/// URI rather than a relative reference. `rel` may be repeated to filter the response to one or
/// more relation types.
///
/// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
/// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RequestParams {
    /// The decoded WebFinger resource query target.
    pub(crate) resource: Resource,

    /// The decoded relation filters, preserving the client's repeated-key order.
    pub(crate) rel: Vec<String>,
}

impl FromStr for RequestParams {
    type Err = RequestParamsError;

    /// Parses WebFinger query parameters using the protocol's RFC-defined shape.
    ///
    /// WebFinger's query shape is defined by RFC 7033 on top of RFC 3986: exactly one `resource`,
    /// repeated `rel` parameters, and percent-encoded URI query values where `+` remains a literal
    /// plus. Framework query extractors are usually form-style serde deserializers, which are
    /// cleaner for ordinary application queries but do not express those protocol details directly.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>,
    /// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>,
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>, and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.4>.
    fn from_str(query: &str) -> Result<Self, Self::Err> {
        let mut resource = None;
        let mut rel = Vec::new();

        for parameter in query.split('&').filter(|parameter| !parameter.is_empty()) {
            let (key, value) = parameter.split_once('=').unwrap_or((parameter, ""));
            let key = decode_query_param(key)?;
            let value = decode_query_param(value)?;

            match key.as_str() {
                "resource" if resource.is_none() => resource = Some(value),
                "resource" => return Err(RequestParamsError::MultipleResources),
                "rel" => rel.push(value),
                _ => {}
            }
        }

        let resource = resource
            .ok_or(RequestParamsError::MissingResource)?
            .parse()?;
        Ok(RequestParams { resource, rel })
    }
}

/// Decodes one RFC 3986 query parameter component.
///
/// The `percent-encoding` crate leaves malformed percent escapes as literal `%` bytes. WebFinger
/// query parsing rejects those malformed escapes before decoding so handlers only see valid
/// RFC 3986 percent-encoded values.
///
/// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
fn decode_query_param(value: &str) -> Result<String, RequestParamsError> {
    validate_percent_escapes(value)?;
    percent_decode_str(value)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| RequestParamsError::InvalidPercentEncoding)
}

/// Validates that every percent sign starts a complete hexadecimal escape.
///
/// RFC 3986 section 2.1 defines percent encoding as `%` followed by exactly two hexadecimal digits,
/// so bare percent signs, short escapes, and non-hex escapes are rejected before decoding.
///
/// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
fn validate_percent_escapes(value: &str) -> Result<(), RequestParamsError> {
    let mut bytes = value.as_bytes().iter();
    while let Some(byte) = bytes.next() {
        if *byte != b'%' {
            continue;
        }
        let Some(high) = bytes.next() else {
            return Err(RequestParamsError::InvalidPercentEncoding);
        };
        let Some(low) = bytes.next() else {
            return Err(RequestParamsError::InvalidPercentEncoding);
        };
        if !high.is_ascii_hexdigit() || !low.is_ascii_hexdigit() {
            return Err(RequestParamsError::InvalidPercentEncoding);
        }
    }
    Ok(())
}

/// Errors that can occur while parsing WebFinger query parameters.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum RequestParamsError {
    /// The required `resource` parameter is missing.
    #[error("missing resource parameter")]
    MissingResource,

    /// More than one `resource` parameter was provided.
    #[error("multiple resource parameters")]
    MultipleResources,

    /// A query parameter contains malformed percent encoding or invalid UTF-8 after decoding.
    #[error("invalid percent-encoded query parameter")]
    InvalidPercentEncoding,

    /// The required `resource` parameter is not an absolute URI.
    #[error("invalid resource: {0}")]
    InvalidResource(#[from] ResourceError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    /// Decodes a percent-encoded `acct:` resource.
    ///
    /// RFC 7033 section 4.1 says WebFinger request parameter values are percent-encoded. This test
    /// catches the original class of bug where encoded resource values reached URI parsing without
    /// first being decoded.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn decodes_percent_encoded_resource() {
        let query: RequestParams = "resource=acct%3Abad%40example.org".parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:bad@example.org".parse().unwrap(),
                rel: Vec::new(),
            },
        );
    }

    /// Preserves repeated `rel` parameters in request order.
    ///
    /// RFC 7033 section 4.1 models relation filters as repeated `rel` parameters, not a list value.
    /// This prevents regressions where a map-shaped parser collapses repeated keys and hides valid
    /// filters from handlers.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn preserves_repeated_rel_params() {
        const QUERY: &str = "resource=acct%3Acarol%40example.org&rel=profile&rel=avatar";
        let query: RequestParams = QUERY.parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:carol@example.org".parse().unwrap(),
                rel: vec!["profile".to_string(), "avatar".to_string()],
            },
        );
    }

    /// Decodes percent-encoded relation URIs.
    ///
    /// Relation filters are often URI strings. RFC 3986 section 2.1 percent encoding must be
    /// decoded before handlers compare relation values, otherwise lookups see encoded text
    /// instead of the requested relation.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn decodes_percent_encoded_rel_params() {
        let rel = "http%3A%2F%2Fwebfinger.example%2Frel%2Fprofile-page";
        let query_string = format!("resource=acct%3Acarol%40example.org&rel={rel}");
        let query: RequestParams = query_string.parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:carol@example.org".parse().unwrap(),
                rel: vec!["http://webfinger.example/rel/profile-page".to_string()],
            },
        );
    }

    /// Rejects percent-decoded values that are not valid UTF-8.
    ///
    /// The parser returns owned Rust strings, so RFC 3986 percent-encoded bytes that do not decode
    /// to UTF-8 must fail before they can become replacement characters or handler-visible
    /// data.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn rejects_invalid_utf8_percent_encoded_values() {
        const QUERY: &str = "resource=acct%3Acarol%40example.org&rel=%FF";
        let error = QUERY.parse::<RequestParams>().unwrap_err();

        assert_eq!(error, RequestParamsError::InvalidPercentEncoding);
    }

    /// Rejects malformed percent escape syntax.
    ///
    /// Some percent decoders leave invalid escapes like `%GG` unchanged. RFC 3986 section 2.1
    /// defines percent encoding as `%` followed by two hexadecimal digits, so the parser
    /// rejects malformed escapes before decoding.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn rejects_malformed_percent_escape_syntax() {
        const QUERY: &str = "resource=acct%3Acarol%40example.org&rel=%GG";
        let error = QUERY.parse::<RequestParams>().unwrap_err();

        assert_eq!(error, RequestParamsError::InvalidPercentEncoding);
    }

    /// Rejects incomplete escapes in parameter keys and values.
    ///
    /// Query keys still use RFC 3986 percent encoding. A malformed key cannot be safely ignored
    /// because it may be a misspelled or corrupted `resource` parameter.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn rejects_incomplete_percent_escape_syntax() {
        for query in [
            "resource%=acct%3Acarol%40example.org",
            "resource%4=acct%3Acarol%40example.org",
            "resource=acct%3Acarol%40example.org%",
            "resource=acct%3Acarol%40example.org%4",
        ] {
            let error = query.parse::<RequestParams>().unwrap_err();

            assert_eq!(error, RequestParamsError::InvalidPercentEncoding);
        }
    }

    /// Accepts `resource` in any query parameter position.
    ///
    /// RFC 7033 section 4.1 defines parameter names but not an order. This prevents order-sensitive
    /// parsing where optional `rel` filters or extension parameters before `resource` break
    /// otherwise valid requests.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn resource_parameter_order_does_not_matter() {
        const QUERY: &str = "rel=profile&resource=acct%3Acarol%40example.org";
        let query: RequestParams = QUERY.parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:carol@example.org".parse().unwrap(),
                rel: vec!["profile".to_string()],
            },
        );
    }

    /// Ignores unknown query parameters while preserving the required WebFinger fields.
    ///
    /// RFC 7033 defines `resource` and `rel`; ignoring additional parameters keeps adapters
    /// forwards-compatible without letting extensions alter the parsed request target.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn ignores_unknown_query_params() {
        let query: RequestParams = "resource=acct%3Acarol%40example.org&foo=bar&rel=avatar"
            .parse()
            .unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:carol@example.org".parse().unwrap(),
                rel: vec!["avatar".to_string()],
            },
        );
    }

    /// Keeps encoded `=` and `&` inside the decoded `resource` value.
    ///
    /// Resource URIs may contain their own query strings. The WebFinger query must split on outer
    /// delimiters before decoding, otherwise encoded inner delimiters can be mistaken for WebFinger
    /// parameters and corrupt the target resource.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn encoded_delimiters_stay_inside_resource() {
        let resource = "https%3A%2F%2Fexample.org%2Fprofile%3Fa%3D1%26b%3D2";
        let query_string = format!("resource={resource}");
        let query: RequestParams = query_string.parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "https://example.org/profile?a=1&b=2".parse().unwrap(),
                rel: Vec::new(),
            },
        );
    }

    /// Decodes `%25` to a literal percent sign without decoding the inner escape again.
    ///
    /// WebFinger query parsing performs exactly one RFC 3986 percent-decoding pass. This preserves
    /// target URIs that already contain percent escapes, such as `%20`, after the outer query
    /// encodes their percent signs as `%25`.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[test]
    fn decodes_encoded_percent_once() {
        const QUERY: &str = "resource=https://example.org/profile/a%2520b";
        let query: RequestParams = QUERY.parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "https://example.org/profile/a%20b".parse().unwrap(),
                rel: Vec::new(),
            },
        );
    }

    /// Preserves literal `+` instead of applying form-style space decoding.
    ///
    /// WebFinger request values use RFC 3986 query encoding. A `+` is valid query data there, while
    /// form-style decoders may translate it to a space and change the resource identifier.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.4>.
    #[test]
    fn plus_is_not_decoded_as_space() {
        let query: RequestParams = "resource=acct%3Acarol+tag%40example.org".parse().unwrap();

        assert_eq!(
            query,
            RequestParams {
                resource: "acct:carol+tag@example.org".parse().unwrap(),
                rel: Vec::new(),
            },
        );
    }

    /// Rejects ambiguous requests with more than one `resource` query target.
    ///
    /// RFC 7033 section 4.2 requires exactly one `resource` parameter. Accepting duplicates would
    /// force adapters to choose one target and hide an ambiguous client request.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[test]
    fn rejects_multiple_resource_params() {
        const QUERY: &str =
            "resource=acct%3Acarol%40example.org&resource=acct%3Aalice%40example.org";
        let error = QUERY.parse::<RequestParams>().unwrap_err();

        assert_eq!(error, RequestParamsError::MultipleResources);
    }

    /// Rejects requests that omit the required `resource` parameter.
    ///
    /// RFC 7033 section 4.2 treats absent `resource` parameters as bad requests. This keeps adapter
    /// code from inventing a default target or depending on framework-specific deserialization
    /// errors.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[test]
    fn rejects_missing_resource_param() {
        let error = "rel=profile".parse::<RequestParams>().unwrap_err();

        assert_eq!(error, RequestParamsError::MissingResource);
    }

    /// Rejects relative references in the `resource` parameter.
    ///
    /// RFC 7033 section 4.1 says the WebFinger query target is a URI. RFC 3986 distinguishes a URI
    /// from a relative reference by the required scheme, so values like `carol`, `/relative`,
    /// `../x`, and the empty string are malformed WebFinger resources.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-4.1>.
    #[test]
    fn rejects_relative_resource_references() {
        for query in [
            "resource=carol",
            "resource=/relative",
            "resource=../x",
            "resource=",
        ] {
            let error = query.parse::<RequestParams>().unwrap_err();

            assert_eq!(
                error,
                RequestParamsError::InvalidResource(ResourceError::RelativeReference)
            );
        }
    }

    /// Exposes the resource parse error as the source of invalid-resource query failures.
    ///
    /// Adapters render all malformed queries as bad requests, but preserving the source keeps
    /// logs and direct error handling specific enough to explain what was invalid.
    #[test]
    fn invalid_resource_error_exposes_source_error() {
        let error = "resource=/relative".parse::<RequestParams>().unwrap_err();

        assert_eq!(
            error.source().map(ToString::to_string),
            Some("resource must be an absolute URI".to_string())
        );
    }
}
