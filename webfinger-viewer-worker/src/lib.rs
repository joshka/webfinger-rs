//! Cloudflare Worker runtime for the WebFinger viewer.
//!
//! This crate owns the Cloudflare Worker HTTP boundary, Worker fetch adapter, and wasm logging
//! setup. Shared route policy, htmx behavior, lookup construction, and Askama templates live in
//! `webfinger-viewer`. Native Axum deployment lives in `webfinger-viewer-axum`.

mod observability;
mod server;

pub use crate::server::serve;

/// Cloudflare Worker fetch entrypoint.
///
/// Keep this shim small. Future Worker request/response plumbing should live in `server`; shared
/// viewer behavior belongs in `webfinger-viewer`.
#[worker::event(fetch)]
async fn fetch(
    request: ::worker::Request,
    env: ::worker::Env,
    ctx: ::worker::Context,
) -> ::worker::Result<::worker::Response> {
    observability::init();
    server::serve(request, env, ctx).await
}
