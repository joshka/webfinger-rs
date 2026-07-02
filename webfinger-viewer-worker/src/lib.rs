//! Cloudflare Worker support for a WebFinger viewer and debugging UI.
//!
//! This crate owns the standalone viewer Worker, not the server-side WebFinger responder. It has
//! three internal modules:
//!
//! - `server` handles the Cloudflare Worker HTTP boundary: serving the static UI, accepting
//!   same-page htmx `/api/lookup` requests, rejecting non-htmx or cross-site browser requests, and
//!   returning swappable result fragments.
//! - `lookup` handles WebFinger-specific behavior: parsing the user's resource or full WebFinger
//!   URL, constructing the target `/.well-known/webfinger` request, fetching it from the Worker
//!   runtime, and shaping the debugging payload returned to the UI.
//! - `view` handles rendering: the full page shell, htmx result fragments, and the view models
//!   consumed by Askama templates.
//!
//! The Worker performs target lookups server-side because browser fetches to arbitrary WebFinger
//! endpoints often fail on CORS. The UI can be deployed below a path such as `/webfinger`, while
//! outbound discovery still targets the standard `/.well-known/webfinger` path on the selected
//! resource's host.

mod lookup;
mod server;
mod view;

pub use crate::server::serve;

/// Cloudflare Worker fetch entrypoint.
///
/// Keep this shim small. Future routing, response formatting, and lookup behavior should live in
/// `server`, `lookup`, or `view` so the crate root stays a map of the Worker rather than the
/// implementation.
#[worker::event(fetch)]
async fn fetch(
    request: ::worker::Request,
    env: ::worker::Env,
    ctx: ::worker::Context,
) -> ::worker::Result<::worker::Response> {
    server::serve(request, env, ctx).await
}
