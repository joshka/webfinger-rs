use http::Uri;
use tracing::trace;

use crate::{error::Error, WebFingerRequest, WebFingerResponse};

struct EmptyBody;

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

impl TryFrom<&WebFingerRequest> for reqwest::Request {
    type Error = crate::Error;

    fn try_from(query: &WebFingerRequest) -> Result<reqwest::Request, crate::Error> {
        let request = http::Request::try_from(query)?;
        let request = reqwest::Request::try_from(request)?;
        Ok(request)
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
    pub async fn execute_reqwest(&self) -> Result<WebFingerResponse, Error> {
        let client = reqwest::Client::new();
        self.execute_reqwest_with_client(&client).await
    }

    /// Executes the WebFinger request with a custom reqwest client.
    #[tracing::instrument]
    pub async fn execute_reqwest_with_client(
        &self,
        client: &reqwest::Client,
    ) -> Result<WebFingerResponse, Error> {
        let request = self.try_into()?;
        trace!("request: {:?}", request);
        let response = client.execute(request).await?;
        trace!("response: {:?}", response);
        async_convert::TryFrom::try_from(response).await
    }

    /// Converts the WebFinger request into a reqwest request.
    pub fn try_into_reqwest(&self) -> Result<reqwest::Request, Error> {
        self.try_into()
    }
}

impl WebFingerResponse {
    /// Converts a reqwest response into a WebFinger response.
    pub async fn try_from_reqwest(response: reqwest::Response) -> Result<WebFingerResponse, Error> {
        async_convert::TryFrom::try_from(response).await
    }
}

#[async_convert::async_trait]
impl async_convert::TryFrom<reqwest::Response> for WebFingerResponse {
    type Error = crate::Error;

    async fn try_from(response: reqwest::Response) -> Result<WebFingerResponse, crate::Error> {
        let response = response.error_for_status()?;
        let response = response.json().await?;
        Ok(response)
    }
}
