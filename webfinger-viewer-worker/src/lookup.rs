//! WebFinger lookup construction and result shaping.
//!
//! The viewer accepts either a WebFinger resource identifier such as `acct:alice@example.com` or a
//! full `/.well-known/webfinger` URL. This module is the table of contents for lookup behavior and
//! owns the outbound Worker fetch adapter. Request parsing, deployment policy, and result shaping
//! live in focused child modules so future changes to resource parsing, target policy, redirects,
//! or body capture can be reviewed against smaller tests.

mod policy;
mod request;
mod result;

use futures_util::TryStreamExt;
use tracing::{error, info, instrument};
use worker::{
    Fetch, Headers, Method, Request, RequestInit, RequestRedirect, Response, ResponseBody,
};

pub use policy::LookupPolicy;
pub use request::LookupRequest;
use result::LookupResultParts;
pub use result::{LookupError, LookupResult};

const MAX_BODY_BYTES: usize = 512 * 1024;
const ACCEPT_HEADER: &str = "application/jrd+json, application/json;q=0.9, */*;q=0.1";

/// Fetches the target WebFinger endpoint and returns the browser debugging payload.
///
/// Redirects are deliberately handled with `RequestRedirect::Manual`. Public deployments are
/// same-origin by default, so automatically following target redirects would let a same-origin
/// endpoint pull the Worker across that policy boundary before the final URL could be inspected.
/// Manual mode returns the target `3xx` response and `Location` header as debugging data instead.
#[instrument(skip(request), fields(target_url = %request.target_url()))]
pub async fn fetch_webfinger(request: &LookupRequest) -> Result<LookupResult, LookupError> {
    info!("fetching webfinger resource");
    let request_init = webfinger_request_init()?;
    let worker_request = Request::new_with_init(request.target_url().as_str(), &request_init)?;
    let mut response = Fetch::Request(worker_request).send().await?;
    let status = response.status_code();
    let content_type = response.headers().get("content-type")?;
    let redirect_location = response.headers().get("location")?;
    let body = read_capped_body(&mut response).await?;
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

/// Builds the outbound WebFinger request options used for target fetches.
///
/// Keeping this as a named helper makes the redirect policy testable without a live Worker fetch.
/// Future workers-rs upgrades should keep the invariant: target redirects are returned to the UI,
/// not followed by this Worker.
fn webfinger_request_init() -> Result<RequestInit, LookupError> {
    let mut request_init = RequestInit::new();
    request_init.with_method(Method::Get);
    request_init.with_redirect(target_redirect_mode());

    let headers = Headers::new();
    headers.set("accept", ACCEPT_HEADER)?;
    request_init.with_headers(headers);
    Ok(request_init)
}

/// Returns the redirect mode for target WebFinger fetches.
///
/// This pure helper keeps the security invariant testable on the native test target. The actual
/// `RequestInit` and `Headers` wrappers call wasm-bindgen imports and can only be exercised in the
/// Worker runtime or wasm target checks.
fn target_redirect_mode() -> RequestRedirect {
    RequestRedirect::Manual
}

/// Logs lookup failures at the protocol boundary.
///
/// The server module owns HTTP status mapping, but lookup owns the log message because it can add
/// protocol context without making `server` understand WebFinger internals.
pub fn log_lookup_error(error: &LookupError) {
    error!(?error, "webfinger lookup failed");
}

/// Logs target fetch outcomes.
///
/// The wasm entrypoint installs a console-backed tracing subscriber, so these events appear in
/// Wrangler tail and Cloudflare dashboard logs. Keep the fields low-cardinality enough to filter by
/// status and target host while still preserving the exact URL needed to reproduce a WebFinger
/// debugging failure.
fn log_lookup_result(
    request: &LookupRequest,
    status: u16,
    content_type: Option<&str>,
    truncated: bool,
) {
    info!(
        status,
        target_url = %request.target_url(),
        resource = %request.resource(),
        content_type = content_type.unwrap_or(""),
        truncated,
        "webfinger lookup result",
    );
}

/// Captured target body after enforcing the viewer's response-size limit.
struct CappedBody {
    /// Response body decoded lossily as UTF-8 for display.
    text: String,

    /// True when the target body exceeded `MAX_BODY_BYTES`.
    truncated: bool,
}

/// Reads at most `MAX_BODY_BYTES + 1` bytes from the target response.
///
/// The extra byte is only used to prove truncation. This avoids the previous `Response::bytes()`
/// path, which buffered the entire target response before applying the viewer cap.
async fn read_capped_body(response: &mut Response) -> Result<CappedBody, LookupError> {
    match response.body() {
        ResponseBody::Empty => return Ok(cap_body_bytes(Vec::new())),
        ResponseBody::Body(bytes) => return Ok(cap_body_bytes(bytes.clone())),
        ResponseBody::Stream(_) => {}
    }

    let mut stream = response.stream()?;
    let mut bytes = Vec::new();
    let read_limit = MAX_BODY_BYTES + 1;

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

    Ok(cap_body_bytes(bytes))
}

/// Applies the viewer body limit after a bounded read has completed.
///
/// The caller may pass at most `MAX_BODY_BYTES + 1` bytes. The extra byte records that the target
/// body was larger than the UI will render without preserving more attacker-controlled data.
fn cap_body_bytes(mut bytes: Vec<u8>) -> CappedBody {
    let truncated = bytes.len() > MAX_BODY_BYTES;
    if truncated {
        bytes.truncate(MAX_BODY_BYTES);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();

    CappedBody { text, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_request_policy_uses_manual_redirects() {
        let redirect: &str = target_redirect_mode().into();

        assert_eq!(redirect, "manual");
        assert_eq!(
            ACCEPT_HEADER,
            "application/jrd+json, application/json;q=0.9, */*;q=0.1",
        );
    }

    #[test]
    fn caps_body_bytes_after_limit() {
        let body = cap_body_bytes(vec![b'a'; MAX_BODY_BYTES + 1]);

        assert!(body.truncated);
        assert_eq!(body.text.len(), MAX_BODY_BYTES);
    }

    #[test]
    fn exact_limit_is_not_truncated() {
        let body = cap_body_bytes(vec![b'a'; MAX_BODY_BYTES]);

        assert!(!body.truncated);
        assert_eq!(body.text.len(), MAX_BODY_BYTES);
    }
}
