use http::Uri;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{Error, Rel};

/// A WebFinger request.
///
/// This represents the request portion of a WebFinger query that can be executed against a
/// WebFinger server.
///
/// `Request` stores three pieces of information that map directly to the outgoing request URL:
///
/// - `resource` becomes the `resource=` query parameter.
/// - `host` becomes the HTTPS authority for the request URL.
/// - Each value in `rels` becomes another `rel=` query parameter, in insertion order.
///
/// In other words, this request:
///
/// ```rust
/// use webfinger_rs::WebFingerRequest;
///
/// let request = WebFingerRequest::builder("acct:carol@example.com")?
///     .host("example.com")
///     .rel("http://webfinger.net/rel/profile-page")
///     .rel("http://webfinger.net/rel/avatar")
///     .build();
/// # let _ = request;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// maps to this URL shape:
///
/// ```text
/// https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://webfinger.net/rel/profile-page&rel=http://webfinger.net/rel/avatar
/// ```
///
/// `host` is required when you want to turn the request into an outgoing HTTP request, because the
/// WebFinger endpoint is always built as `https://{host}/.well-known/webfinger?...`. For `acct:`
/// resources, set `host` to the domain that serves WebFinger for that account. In the common case,
/// that is the same domain that appears after `@` in the `acct:` URI.
///
/// `acct:` resources should include the full account URI, such as
/// `acct:carol@example.com`, not just `carol@example.com` or `@carol@example.com`.
///
/// Repeated relation filters are encoded as repeated `rel` query parameters rather than as a
/// comma-separated list.
///
/// See: [RFC 7033 section 4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1) for
/// query-construction rules and parameter encoding.
///
/// See: [RFC 7565 section 3](https://www.rfc-editor.org/rfc/rfc7565.html#section-3) for the
/// `acct:` URI syntax used by account resources.
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::WebFingerRequest;
///
/// let request = WebFingerRequest::builder("acct:carol@example.com")?
///     .host("example.com")
///     .rel("http://webfinger.net/rel/profile-page")
///     .build();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// To execute the query, enable the `reqwest` feature and call `query.execute()`.
///
/// ```rust
/// # #[cfg(feature = "reqwest")]
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # use webfinger_rs::WebFingerRequest;
/// # let request = WebFingerRequest::builder("acct:carol@example.com")?
/// #     .host("example.com")
/// #     .build();
/// let response = request.execute_reqwest().await?;
/// # Ok(())
/// # }
/// ```
///
/// `Request` can be used as an Axum extractor as it implements [`axum::extract::FromRequestParts`].
///
/// ```rust
/// use webfinger_rs::{WebFingerRequest, WebFingerResponse};
///
/// async fn handler(request: WebFingerRequest) -> WebFingerResponse {
///     // ... handle the request ...
/// # WebFingerResponse::new("")
/// }
/// ```
#[serde_as]
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Request {
    /// Query target.
    ///
    /// This is the URI of the resource to query. It will be stored in the `resource` query
    /// parameter.
    ///
    /// For account lookups, use the full `acct:` URI, for example `acct:carol@example.com`.
    ///
    /// See: [RFC 7565 section 3](https://www.rfc-editor.org/rfc/rfc7565.html#section-3).
    ///
    /// TODO: This could be a newtype that represents the resource and makes it easier to extract
    /// the values / parse into the right types (e.g. `acct:` URIs).
    #[serde_as(as = "DisplayFromStr")]
    pub resource: Uri,

    /// The host to query.
    ///
    /// This becomes the HTTPS authority of the final request URL. When converting this request to
    /// an outgoing [`http::Uri`], the crate builds
    /// `https://{host}/.well-known/webfinger?...`.
    ///
    /// Set this explicitly before executing the request or converting it into an outgoing URL.
    /// For `acct:` resources, this is usually the domain part of the account identifier.
    ///
    /// See: [RFC 7033 section 4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1).
    ///
    /// TODO: this might be better as an `Option<Uri>` or `Option<Host>` or something similar. When
    /// the resource has a host part, it should be used unless this field is set.
    pub host: String,

    /// Link relation types
    ///
    /// This is a list of link relation types to query for. Each link relation type will be stored
    /// in its own `rel` query parameter.
    ///
    /// See: [RFC 7033 section 4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1).
    pub rels: Vec<Rel>,
}

impl Request {
    /// Creates a new WebFinger request.
    pub fn new(resource: Uri) -> Self {
        Self {
            host: String::new(),
            resource,
            rels: Vec::new(),
        }
    }

    /// Creates a new [`Builder`] for a WebFinger request.
    pub fn builder<U>(uri: U) -> Result<Builder, Error>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<Error>,
    {
        Builder::new(uri)
    }
}

/// A builder for a WebFinger request.
///
/// This is used to construct a [`Request`] for a WebFinger query.
///
/// # Examples
///
/// ```rust
/// use webfinger_rs::WebFingerRequest;
///
/// let query = WebFingerRequest::builder("acct:carol@example.com")?
///     .host("example.com")
///     .rel("http://webfinger.net/rel/profile-page")
///     .build();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct Builder {
    request: Request,
}

impl Builder {
    /// Creates a new WebFinger request builder.
    ///
    /// This will use the given URI as the resource for the query.
    ///
    /// For account lookups, pass the complete `acct:` URI, such as `acct:carol@example.com`.
    ///
    /// See: [RFC 7565 section 3](https://www.rfc-editor.org/rfc/rfc7565.html#section-3).
    ///
    /// # Errors
    ///
    /// This will return an error if the URI is invalid.
    pub fn new<U>(uri: U) -> Result<Self, Error>
    where
        Uri: TryFrom<U>,
        <Uri as TryFrom<U>>::Error: Into<Error>,
    {
        TryFrom::try_from(uri)
            .map(|uri| Self {
                request: Request::new(uri),
            })
            .map_err(Into::into)
    }

    /// Sets the host for the query.
    ///
    /// This host is used as the authority in the final HTTPS request URL.
    ///
    /// See: [RFC 7033 section 4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1).
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.request.host = host.into();
        self
    }

    /// Adds a link relation type to the query.
    ///
    /// Each call appends another `rel` query parameter to the outgoing request URL.
    ///
    /// See: [RFC 7033 section 4.1](https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// let query = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .build();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn rel<R: Into<Rel>>(mut self, rel: R) -> Self {
        self.request.rels.push(rel.into());
        self
    }

    /// Builds the WebFinger request.
    ///
    /// # Examples
    ///
    /// Build a request for an `acct:` resource and inspect the final URL:
    ///
    /// ```rust
    /// use http::Uri;
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .build();
    ///
    /// let uri = Uri::try_from(&request)?;
    ///
    /// assert_eq!(
    ///     uri.to_string(),
    ///     "https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://webfinger.net/rel/profile-page",
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Multiple relation filters become repeated `rel` query parameters:
    ///
    /// ```rust
    /// use http::Uri;
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .rel("http://webfinger.net/rel/avatar")
    ///     .build();
    ///
    /// let uri = Uri::try_from(&request)?;
    ///
    /// assert_eq!(
    ///     uri.to_string(),
    ///     "https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://webfinger.net/rel/profile-page&rel=http://webfinger.net/rel/avatar",
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn build(self) -> Request {
        self.request
    }
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::*;

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.1
    #[test]
    fn example_3_1() {
        let resource = "acct:carol@example.com".parse().unwrap();
        let rel = Rel::from("http://openid.net/specs/connect/1.0/issuer");
        let host = "example.com".parse().unwrap();
        let query = Request {
            host,
            resource,
            rels: vec![rel],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // `"/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fopenid.net%
        // 2Fspecs%2Fconnect%2F1.0%2Fissuer"`
        assert_eq!(
            uri.to_string(),
            "https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://openid.net/specs/connect/1.0/issuer",
        );
    }

    /// https://www.rfc-editor.org/rfc/rfc7033.html#section-3.2
    #[test]
    fn example_3_2() {
        let resource = "http://blog.example.com/article/id/314".parse().unwrap();
        let query = Request {
            host: "blog.example.com".parse().unwrap(),
            resource,
            rels: vec![],
        };
        let uri = Uri::try_from(&query).unwrap();

        // The RFC unnecessarily percent-encodes this to:
        // /.well-known/webfinger?resource=http%3A%2F%2Fblog.example.com%2Farticle%2Fid%2F314
        assert_eq!(
            uri.to_string(),
            "https://blog.example.com/.well-known/webfinger?resource=http://blog.example.com/article/id/314",
        );
    }
}
