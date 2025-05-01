use http::Uri;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{Error, Rel};

/// A WebFinger request.
///
/// This represents the request portion of a WebFinger query that can be executed against a
/// WebFinger server.
///
/// See [RFC 7033 Section 4](https://www.rfc-editor.org/rfc/rfc7033.html#section-4) for more
/// information.
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
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # use webfinger_rs::WebFingerRequest;
/// # let request = WebFingerRequest::builder("acct:carol@example.com")?.build();
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
#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    /// Query target.
    ///
    /// This is the URI of the resource to query. It will be stored in the `resource` query
    /// parameter.
    ///
    /// TODO: This could be a newtype that represents the resource and makes it easier to extract
    /// the values / parse into the right types (e.g. `acct:` URIs).
    #[serde_as(as = "DisplayFromStr")]
    pub resource: Uri,

    /// The host to query
    ///
    /// TODO: this might be better as an `Option<Uri>` or `Option<Host>` or something similar. When
    /// the resource has a host part, it should be used unless this field is set.
    pub host: String,

    /// Link relation types
    ///
    /// This is a list of link relation types to query for. Each link relation type will be stored
    /// in a `rel` query parameter.
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

    /// Creates a new [`WebFingerBuilder`] for a WebFinger request.
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
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.request.host = host.into();
        self
    }

    /// Adds a link relation type to the query.
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
