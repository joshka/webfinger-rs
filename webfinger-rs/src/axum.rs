use axum::{body::Body, http::Response as AxumResponse, response::IntoResponse};
use http::{header, HeaderValue};

use crate::Response;

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");

impl IntoResponse for Response {
    fn into_response(self) -> AxumResponse<Body> {
        let body = serde_json::to_string(&self).unwrap();
        let mut response = AxumResponse::new(Body::from(body));
        let headers = response.headers_mut();
        headers.insert(header::CONTENT_TYPE, JRD_CONTENT_TYPE);
        response
    }
}
