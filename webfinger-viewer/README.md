# WebFinger Viewer

`webfinger-viewer` is the shared viewer library for the repository's WebFinger debugging UI. It
owns runtime-neutral route policy, lookup validation, htmx response behavior, Askama templates,
assets, view models, and runtime-neutral config.

Runtime crates adapt the shared viewer behavior to concrete platforms:

- [`webfinger-viewer-axum`](../webfinger-viewer-axum/README.md) runs the viewer as a native Axum
  application.
- [`webfinger-viewer-worker`](../webfinger-viewer-worker/README.md) runs the same viewer as a
  Cloudflare Worker.

This crate is not directly runnable. Use the Axum runtime for local development:

```console
cargo run -p webfinger-viewer-axum
```

## Behavior

The browser UI calls a Rust runtime for lookups. The runtime then fetches the target
`/.well-known/webfinger` endpoint server-side, which avoids browser CORS failures while keeping
public deployments constrained to the hostname that served the viewer.

Public deployments are same-origin by default: a viewer served from `https://example.com/webfinger`
can inspect `https://example.com/.well-known/webfinger`, but it rejects lookups for unrelated
hosts. Local sessions are the exception. When the viewer itself is served from `localhost`,
`127.0.0.1`, or `::1`, off-origin lookups are allowed so a local viewer can inspect public
resources such as `acct:joshka@hachyderm.io` and another local server on a port such as `8787`.

Plain loopback resources such as `acct:alice@localhost` derive
`http://localhost:8787/.well-known/webfinger`, and loopback full URLs entered as
`https://localhost:8787/...` are normalized to `http://localhost:8787/...` because local debugging
servers use plain HTTP.

The local responder port is configurable through the runtime-neutral `ViewerConfig` model. The
checked-in default config uses `8787`, while `viewer.example.toml` uses `8790` so local runs can
demonstrate that config affects lookup behavior.

## Features

- Accepts same-origin `acct:` resources such as `acct:alice@example.com` when deployed on
  `example.com`.
- Accepts full WebFinger URLs such as
  `https://example.com/.well-known/webfinger?resource=acct:alice@example.com`.
- Accepts arbitrary resource hosts during local development, including `acct:joshka@hachyderm.io`.
- Uses `webfinger-rs::Resource` validation and host extraction for URI resources, with a viewer
  fallback for `acct:` identifiers.
- Maps local resource identifiers such as `acct:alice@localhost` and `acct:alice@127.0.0.1` to the
  configured local responder port.
- Accepts full loopback WebFinger URLs during local development, such as
  `http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost`.
- Adds optional repeated `rel` filters to the target request.
- Shows HTTP status, content type, request URL, redirect `Location`, parsed JRD JSON, and raw
  response text.
- Renders JRD aliases, properties, and links in a readable layout.
- Provides a copyable `curl -i` command for the exact target lookup.

## Implementation Map

The viewer is split so future UI or protocol changes have one clear owner:

- `src/app.rs` owns shared route policy, htmx response behavior, headers, browser history URLs, and
  lookup form handling.
- `src/config.rs` owns runtime-neutral TOML config. Native runtimes use `[server]` defaults, while
  shared lookup policy uses `[lookup].local_responder_port`.
- `src/lookup.rs` owns WebFinger behavior. It parses viewer input, constructs the target
  `/.well-known/webfinger` URL, enforces same-origin or local-loopback policy, captures transport
  metadata, and returns the debugging payload.
- `src/view.rs` owns view models and renders Askama templates. It embeds `assets/app.css`,
  `assets/app.js`, and vendored `assets/vendor/htmx.min.js` into the page so deploys do not need
  separate asset routes and Cargo-only CI does not need npm packages installed. The vendored htmx
  file should match the pinned `htmx.org` version in the root `package.json` and
  `package-lock.json`.
- `templates/page.html` is the full page shell. `templates/lookup_result.html` and
  `templates/lookup_error.html` are htmx fragments swapped into `#results`.

htmx form submissions receive HTML fragments. Viewer-level failures such as malformed resources or
runtime fetch errors intentionally return `200` for htmx requests so the browser swaps the error
panel. `/api/lookup` is not a public JSON API: non-htmx callers receive `404`, and browser requests
with `Sec-Fetch-Site: cross-site` receive `403`. Those checks are defense in depth rather than
authentication; direct clients can spoof htmx and Fetch Metadata headers. They keep the supported
contract narrow and avoid CORS-enabled use of the viewer as a general server-side lookup endpoint.

Lookup input is bounded before the runtime fetches a target: resource strings, relation filter
count, relation filter length, final target URL length, target policy, and captured response body
size all have explicit limits. Response headers also set a restrictive baseline for browser use,
including
`Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, and `Permissions-Policy`.
The CSP allows inline script and style because the viewer embeds htmx, CSS, and local JavaScript in
one path-mounted page response; it still blocks remote scripts, framing, base URI changes, and
cross-origin connections.

Host inference deliberately follows the library resource model. HTTP and HTTPS resources use the
cached host from `webfinger-rs::Resource`; `acct:` resources use the domain after the final `@`.
Other valid resource schemes, such as `urn:`, do not imply a host, so the viewer asks for a full
`/.well-known/webfinger` URL when it cannot infer where to send the lookup.

## Configuration

The shared config schema is TOML:

```toml
[server]
host = "127.0.0.1"
port = 8788

[lookup]
local_responder_port = 8787
```

`[server]` is available to native runtimes that bind a socket. `[lookup]` is runtime-neutral and is
used by shared request construction. For example, setting `local_responder_port = 8790` makes
`acct:alice@localhost` derive
`http://localhost:8790/.well-known/webfinger?resource=acct%3Aalice%40localhost` during local
viewer sessions.

## Validation

Useful checks for this crate:

```console
cargo fmt --all --check
cargo test -p webfinger-viewer
markdownlint-cli2 --no-globs webfinger-viewer/README.md
```
