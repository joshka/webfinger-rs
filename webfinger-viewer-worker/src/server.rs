//! Cloudflare Worker HTTP adapter for the viewer.
//!
//! This module knows about Cloudflare Worker request and response types. Shared route policy,
//! static UI delivery, htmx response behavior, and lookup rendering live in `app` so the same
//! viewer can run under both Worker and native Axum runtimes.

use ::worker::{
    Context, Env, Fetch, Headers, Method, Request, RequestInit, RequestRedirect, Response,
    ResponseBody,
};
use futures_util::TryStreamExt;
use tracing::{info, instrument};
use url::form_urlencoded;
use webfinger_viewer::app::{
    self, LookupForm, MAX_LOOKUP_FORM_BYTES, ViewerHeader, ViewerResponse,
};
use webfinger_viewer::lookup::{
    ACCEPT_HEADER, LookupError, LookupRequest, LookupResult, LookupResultParts, cap_body_bytes,
    log_lookup_result,
};

/// Serves one Cloudflare Worker request.
pub async fn serve(request: Request, _env: Env, _ctx: Context) -> worker::Result<Response> {
    let method = worker_method_to_http(request.method());
    let url = request.url()?;

    if app::is_lookup_path(&url) {
        return serve_lookup(request, method, url).await;
    }

    worker_response(app::serve_page_or_error(&method, &url))
}

async fn serve_lookup(
    mut request: Request,
    method: http::Method,
    url: url::Url,
) -> worker::Result<Response> {
    let is_htmx_request = request.headers().get("hx-request")?.is_some();
    let is_cross_site_request = matches!(
        request.headers().get("sec-fetch-site")?.as_deref(),
        Some("cross-site")
    );
    if let Err(response) =
        app::lookup_preflight(&method, &url, is_htmx_request, is_cross_site_request)
    {
        return worker_response(response);
    }

    let form = match worker_lookup_form(&mut request).await? {
        Ok(form) => form,
        Err(response) => return worker_response(response),
    };
    let response = app::serve_lookup(&url, form, |request| async move {
        fetch_webfinger_worker(&request).await
    })
    .await;
    worker_response(response)
}

/// Fetches the target WebFinger endpoint with the Cloudflare Worker runtime.
///
/// Redirects are deliberately handled with `RequestRedirect::Manual`. Public deployments are
/// same-origin by default, so automatically following target redirects would let a same-origin
/// endpoint pull the Worker across that policy boundary before the final URL could be inspected.
/// Manual mode returns the target `3xx` response and `Location` header as debugging data instead.
#[instrument(skip(request), fields(target_url = %request.target_url()))]
async fn fetch_webfinger_worker(request: &LookupRequest) -> Result<LookupResult, LookupError> {
    info!("fetching webfinger resource");
    let request_init = webfinger_request_init()?;
    let worker_request = Request::new_with_init(request.target_url().as_str(), &request_init)
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    let mut response = Fetch::Request(worker_request)
        .send()
        .await
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    let status = response.status_code();
    let content_type = response
        .headers()
        .get("content-type")
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    let redirect_location = response
        .headers()
        .get("location")
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    let body = read_capped_worker_body(&mut response).await?;
    log_lookup_result(request, status, content_type.as_deref(), body.truncated);

    Ok(LookupResult::new(LookupResultParts {
        request_url: request.target_url().to_string(),
        redirect_location,
        resource: request.resource().to_string(),
        rels: request.rels().to_vec(),
        status,
        content_type,
        body: body.text,
        truncated: body.truncated,
    }))
}

fn webfinger_request_init() -> Result<RequestInit, LookupError> {
    let mut request_init = RequestInit::new();
    request_init.with_method(Method::Get);
    request_init.with_redirect(RequestRedirect::Manual);

    let headers = Headers::new();
    headers
        .set("accept", ACCEPT_HEADER)
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    request_init.with_headers(headers);
    Ok(request_init)
}

async fn read_capped_worker_body(
    response: &mut Response,
) -> Result<webfinger_viewer::lookup::CappedBody, LookupError> {
    match response.body() {
        ResponseBody::Empty => return Ok(cap_body_bytes(Vec::new())),
        ResponseBody::Body(bytes) => return Ok(cap_body_bytes(bytes.clone())),
        ResponseBody::Stream(_) => {}
    }

    let mut stream = response
        .stream()
        .map_err(|error| LookupError::transport("Worker fetch", error))?;
    let mut bytes = Vec::new();
    let read_limit = webfinger_viewer::lookup::MAX_BODY_BYTES + 1;

    while bytes.len() < read_limit {
        let Some(chunk) = stream
            .try_next()
            .await
            .map_err(|error| LookupError::transport("Worker fetch", error))?
        else {
            break;
        };
        let remaining = read_limit - bytes.len();
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(cap_body_bytes(bytes))
}

/// Reads the htmx form body from the Worker request.
///
/// The viewer form posts `application/x-www-form-urlencoded` fields. Reading the stream directly
/// keeps the Worker adapter aligned with Axum's bounded form parsing instead of buffering an
/// unbounded body through `Request::form_data`.
async fn worker_lookup_form(
    request: &mut Request,
) -> worker::Result<Result<LookupForm, ViewerResponse>> {
    let bytes = match read_capped_worker_form_body(request).await? {
        Ok(bytes) => bytes,
        Err(response) => return Ok(Err(response)),
    };

    Ok(Ok(parse_lookup_form(&bytes)))
}

async fn read_capped_worker_form_body(
    request: &mut Request,
) -> worker::Result<Result<Vec<u8>, ViewerResponse>> {
    let mut stream = match request.stream() {
        Ok(stream) => stream,
        Err(worker::Error::RustError(error)) if error == "no body for request" => {
            return Ok(Ok(Vec::new()));
        }
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    let read_limit = MAX_LOOKUP_FORM_BYTES + 1;

    while bytes.len() < read_limit {
        let Some(chunk) = stream.try_next().await? else {
            break;
        };
        let remaining = read_limit - bytes.len();
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    if bytes.len() > MAX_LOOKUP_FORM_BYTES {
        return Ok(Err(oversized_lookup_form_response()));
    }

    Ok(Ok(bytes))
}

fn oversized_lookup_form_response() -> ViewerResponse {
    ViewerResponse {
        status: 413,
        headers: Vec::new(),
        body: format!("form body is too large; maximum is {MAX_LOOKUP_FORM_BYTES} bytes")
            .into_bytes(),
    }
}

fn parse_lookup_form(bytes: &[u8]) -> LookupForm {
    let mut resource = None;
    let mut rels = Vec::new();
    for (key, value) in form_urlencoded::parse(bytes) {
        if key == "resource" {
            resource = Some(value.into_owned());
        } else if key == "rel" {
            rels.push(value.into_owned());
        }
    }

    LookupForm { resource, rels }
}

fn worker_response(response: ViewerResponse) -> worker::Result<Response> {
    let parts = WorkerResponseParts::from(response);
    let mut builder = Response::builder().with_status(parts.status);
    for header in parts.headers {
        builder = builder.with_header(header.name, &header.value)?;
    }
    Ok(builder.fixed(parts.body))
}

#[derive(Debug, PartialEq, Eq)]
struct WorkerResponseParts {
    status: u16,
    headers: Vec<ViewerHeader>,
    body: Vec<u8>,
}

impl From<ViewerResponse> for WorkerResponseParts {
    fn from(response: ViewerResponse) -> Self {
        Self {
            status: response.status,
            headers: response.headers,
            body: response.body,
        }
    }
}

fn worker_method_to_http(method: worker::Method) -> http::Method {
    match method {
        worker::Method::Head => http::Method::HEAD,
        worker::Method::Get => http::Method::GET,
        worker::Method::Post => http::Method::POST,
        worker::Method::Put => http::Method::PUT,
        worker::Method::Patch => http::Method::PATCH,
        worker::Method::Delete => http::Method::DELETE,
        worker::Method::Options => http::Method::OPTIONS,
        worker::Method::Connect => http::Method::CONNECT,
        worker::Method::Trace => http::Method::TRACE,
        worker::Method::Report => http::Method::from_bytes(b"REPORT").expect("REPORT is valid"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_worker_methods_to_http_methods() {
        assert_eq!(
            worker_method_to_http(worker::Method::Head),
            http::Method::HEAD
        );
        assert_eq!(
            worker_method_to_http(worker::Method::Get),
            http::Method::GET
        );
        assert_eq!(
            worker_method_to_http(worker::Method::Post),
            http::Method::POST
        );
        assert_eq!(
            worker_method_to_http(worker::Method::Report),
            http::Method::from_bytes(b"REPORT").unwrap(),
        );
    }

    #[test]
    fn worker_response_parts_map_status_headers_and_body() {
        let parts = WorkerResponseParts::from(ViewerResponse {
            status: 418,
            headers: vec![ViewerHeader {
                name: "x-viewer-test",
                value: "ok".to_string(),
            }],
            body: b"teapot".to_vec(),
        });

        assert_eq!(parts.status, 418);
        assert_eq!(parts.headers[0].name, "x-viewer-test");
        assert_eq!(parts.headers[0].value, "ok");
        assert_eq!(parts.body, b"teapot");
    }

    #[test]
    fn parses_url_encoded_lookup_form() {
        let form = parse_lookup_form(
            b"resource=acct%3Aalice%40example.com&rel=self&rel=http%3A%2F%2Fexample.com%2Frel",
        );

        assert_eq!(form.resource, Some("acct:alice@example.com".to_string()));
        assert_eq!(form.rels, vec!["self", "http://example.com/rel"],);
    }

    #[test]
    fn oversized_lookup_form_response_names_limit() {
        let response = oversized_lookup_form_response();
        let body = String::from_utf8(response.body).unwrap();

        assert_eq!(response.status, 413);
        assert!(body.contains(&MAX_LOOKUP_FORM_BYTES.to_string()));
    }
}
