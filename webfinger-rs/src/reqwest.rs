use http::Uri;
use tracing::trace;

use crate::{error::Error, WebFingerRequest, WebFingerResponse};

struct EmptyBody;

#[cfg(feature = "reqwest")]
impl From<EmptyBody> for reqwest::Body {
    fn from(_: EmptyBody) -> reqwest::Body {
        reqwest::Body::default()
    }
}

impl TryFrom<&WebFingerRequest> for http::Request<EmptyBody> {
    type Error = http::Error;

    fn try_from(query: &WebFingerRequest) -> Result<http::Request<EmptyBody>, http::Error> {
        let uri = Uri::try_from(query)?;
        http::Request::builder()
            .method("GET")
            .uri(uri)
            .body(EmptyBody)
    }
}

impl WebFingerRequest {
    /// Executes the WebFinger request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use webfinger_rs::Request;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let query = Request::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .build();
    /// let response = query.execute().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument]
    pub async fn execute(&self) -> Result<WebFingerResponse, Error> {
        let client = reqwest::Client::new();
        let request = http::Request::try_from(self)?;
        let request = reqwest::Request::try_from(request)?;
        let response = client.execute(request).await?;
        trace!("response: {:?}", response);
        let response = response.error_for_status()?;
        let body = response.json().await?;
        Ok(body)
    }
}
