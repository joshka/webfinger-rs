# WebFinger Viewer Worker

`webfinger-viewer-worker` is the Cloudflare Worker runtime for the WebFinger viewer. It adapts
Worker requests, responses, server-side `fetch()`, and console-backed logging to the shared
[`webfinger-viewer`](../webfinger-viewer/README.md) crate.

Use [`webfinger-viewer-axum`](../webfinger-viewer-axum/README.md) for the native Axum runtime:

```console
cargo run -p webfinger-viewer-axum
```

## Deploy

The Worker can be mounted below a path such as `/webfinger`. It still queries the standard target
path on the same host, such as `https://example.com/.well-known/webfinger`.

The checked-in `wrangler.viewer.toml` config enables Cloudflare's
`global_fetch_strictly_public` compatibility flag. Public deployments need this because the viewer
performs server-side `fetch()` calls back to the same zone. Without the flag, Cloudflare can route
that subrequest to the zone origin while ignoring same-zone Worker routes, which appears in the
viewer as a target `522` even though the same `/.well-known/webfinger` URL works from a browser or
local `curl`.

1. Install the local JavaScript tooling:

```console
npm install
```

1. Run a dry-run deploy:

```console
npm run deploy:viewer:dry-run
```

1. Deploy to the temporary Workers route:

```console
npm run deploy:viewer
```

1. Deploy-test on a site route without committing site-specific config:

```console
npx wrangler deploy --config wrangler.viewer.toml \
  --route 'example.com/webfinger' \
  --route 'example.com/webfinger/*'
```

The route commands require the hostname's zone to be in the Cloudflare account and DNS to be set up
for that hostname. The checked-in `wrangler.viewer.toml` intentionally does not contain Josh-specific
routes or envs because this crate is a reusable tool, not a single-site deploy.

The root Deploy to Cloudflare button is reserved for the service Worker because Cloudflare deploy
buttons do not fully support this workspace as a multi-Worker monorepo. Deploy the viewer with the
explicit viewer scripts above.

Cloudflare's Git deploy environment may provide Node and npm without Rust. The checked-in
`wrangler.viewer.toml` build hook runs `scripts/build-webfinger-viewer-worker.sh`, which installs a
minimal Rust toolchain only when `cargo` is missing, adds the Wasm target, and then runs
`worker-build`. Local developers with Rust already installed use their existing toolchain.
The repository intentionally does not use a root `rust-toolchain.toml`: the library docs workflow
uses nightly for docs.rs-only Rustdoc features, while the Worker deploy hook can bootstrap stable
Rust by itself.

## Observability

`wrangler.viewer.toml` enables Cloudflare Worker observability. The Worker installs a console-backed
`tracing` subscriber in wasm builds, so request decisions and lookup results appear in Wrangler tail
and Cloudflare dashboard logs. Useful log events include:

- `webfinger viewer request` with `method`, `path`, and a stable `outcome` value.
- `webfinger lookup result` with target status, target URL, resource, content type, and truncation
  state.
- lookup input and Worker-fetch errors at error level.

## Local Worker Development

Run Wrangler locally:

```console
npm run dev:viewer
```

Then open the local Wrangler URL. Local loopback viewer sessions allow off-origin lookups, including
public resources and another local WebFinger server on port `8787`.

## Implementation Map

- `src/lib.rs` owns the Worker fetch event and installs wasm logging.
- `src/server.rs` owns Worker request extraction, response construction, form parsing, and the
  Worker `fetch()` adapter.
- [`webfinger-viewer`](../webfinger-viewer/README.md) owns the shared route policy, lookup
  validation, htmx fragments, templates, and assets.
- [`webfinger-viewer-worker`](README.md) does not depend on
  [`webfinger-viewer-axum`](../webfinger-viewer-axum/README.md); native startup and reqwest-backed
  fetch live only in the Axum crate.

## Validation

Useful checks for this crate:

```console
cargo test -p webfinger-viewer-worker
cargo check -p webfinger-viewer-worker --target wasm32-unknown-unknown
markdownlint-cli2 --no-globs webfinger-viewer-worker/README.md
npm run deploy:viewer:dry-run
```

`npm run deploy:viewer:dry-run` runs `worker-build --release` through the
`wrangler.viewer.toml` build hook.
