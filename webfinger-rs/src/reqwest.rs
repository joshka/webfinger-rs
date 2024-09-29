use http::Uri;
use tracing::debug;

use crate::{Error, Request, Response};

struct EmptyBody;

#[cfg(feature = "reqwest")]
impl From<EmptyBody> for reqwest::Body {
    fn from(_: EmptyBody) -> reqwest::Body {
        reqwest::Body::default()
    }
}

impl TryFrom<&Request> for http::Request<EmptyBody> {
    type Error = http::Error;

    fn try_from(query: &Request) -> Result<http::Request<EmptyBody>, http::Error> {
        let uri = Uri::try_from(query)?;
        http::Request::builder()
            .method("GET")
            .uri(uri)
            .body(EmptyBody)
    }
}

impl Request {
    #[tracing::instrument]
    pub async fn fetch(&self) -> Result<Response, Error> {
        let client = reqwest::Client::new();
        let request = http::Request::try_from(self)?;
        let request = reqwest::Request::try_from(request)?;
        let response = client.execute(request).await?;
        debug!("response: {:?}", response);
        let response = response.error_for_status()?;
        let body = response.text().await?;
        debug!(body, "response body");
        let response = serde_json::from_str(&body)?;
        // let response = response.json().await?;
        Ok(response)
    }
}
