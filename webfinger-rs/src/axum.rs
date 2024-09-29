use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Query},
    http::Response as AxumResponse,
    response::IntoResponse,
    Json,
};
use http::{
    header::{self, HOST},
    request::Parts,
    HeaderValue,
};
use tracing::debug;

use crate::{LinkRelationType, Request, Response};

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");

impl IntoResponse for Response {
    fn into_response(self) -> AxumResponse<Body> {
        let mut response = Json(self).into_response();
        let headers = response.headers_mut();
        headers.insert(header::CONTENT_TYPE, JRD_CONTENT_TYPE);
        response
    }
}

#[derive(Debug, serde::Deserialize)]
struct RequestParams {
    resource: String,
    #[serde(default)]
    rel: Vec<String>,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Request {
    type Rejection = AxumResponse<Body>;

    // TODO simplify this
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        debug!("request parts: {:?}", parts);
        let query = Query::<RequestParams>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        let host = parts
            .uri
            .host()
            .or_else(|| parts.headers.get(HOST).and_then(|host| host.to_str().ok()))
            .ok_or(
                AxumResponse::builder()
                    .status(400)
                    .body(Body::from("missing host"))
                    .unwrap(),
            )?;
        let host = host.to_string();
        let resource = query.resource.parse().map_err(|err| {
            AxumResponse::builder()
                .status(400)
                .body(Body::from(format!("invalid resource: {}", err)))
                .unwrap()
        })?;
        let link_relation_types = query
            .rel
            .clone()
            .into_iter()
            .map(LinkRelationType::from)
            .collect();
        Ok(Request {
            host,
            resource,
            link_relation_types,
        })
    }
}
