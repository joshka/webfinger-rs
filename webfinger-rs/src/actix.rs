use std::{future::Future, pin::Pin};

use actix_web::{dev::Payload, web::Json, FromRequest, HttpRequest, HttpResponse, Responder};
use tracing::trace;

use crate::{WebFingerRequest, WebFingerResponse};

impl Responder for WebFingerResponse {
    type Body = <Json<WebFingerResponse> as Responder>::Body;

    fn respond_to(self, _request: &HttpRequest) -> HttpResponse<Self::Body> {
        Json(self).respond_to(_request)
    }
}

impl FromRequest for WebFingerRequest {
    type Error = actix_web::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        trace!(?req, "extracting WebFingerRequest from request");
        let host = req
            .uri()
            .host()
            .or_else(|| req.headers().get("host").and_then(|h| h.to_str().ok()))
            .map(|h| h.to_string());
        let resource = req
            .query_string()
            .split('&')
            .find_map(|param| param.split_once('=').filter(|(key, _)| *key == "resource"))
            .map(|(_, value)| value.to_string());
        let rels_from_query: Vec<_> = req
            .query_string()
            .split('&')
            .filter_map(|param| param.split_once('=').filter(|(key, _)| *key == "rel"))
            .map(|(_, value)| value.to_string())
            .collect();
        Box::pin(async move {
            let resource = resource.ok_or(actix_web::error::ErrorBadRequest("missing resource"))?;
            let host = host.ok_or(actix_web::error::ErrorBadRequest("missing host"))?;
            let mut request_builder = WebFingerRequest::builder(resource).unwrap().host(host);
            for rel in rels_from_query {
                request_builder = request_builder.rel(rel);
            }
            Ok(request_builder.build())
        })
    }
}
