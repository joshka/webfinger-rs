# WebFinger Viewer Worker

[![Deploy to Cloudflare][deploy-button]][deploy-url]

`webfinger-viewer-worker` is a Rust Cloudflare Worker that serves a WebFinger viewer and debugging
UI. It is separate from the server-side WebFinger responder Worker and does not read responder
configuration.

The browser UI calls this Worker for lookups. The Worker then fetches the target
`/.well-known/webfinger` endpoint server-side, which avoids browser CORS failures while keeping
public deployments constrained to the hostname that served the viewer.

Public deployments are same-origin by default: a viewer served from `https://example.com/webfinger`
can inspect `https://example.com/.well-known/webfinger`, but it rejects lookups for unrelated
hosts. Local Wrangler sessions are the exception. When the viewer itself is served from
`localhost`, `127.0.0.1`, or `::1`, full loopback WebFinger URLs are allowed so a local viewer on
port `8788` can inspect another local Worker on a port such as `8787`. In a deployed Cloudflare
Worker, `localhost` refers to the Worker runtime environment, not the developer machine.

## Features

- Accepts same-origin `acct:` resources such as `acct:alice@example.com` when deployed on
  `example.com`.
- Accepts full WebFinger URLs such as
  `https://example.com/.well-known/webfinger?resource=acct:alice@example.com`.
- Accepts full loopback WebFinger URLs during local Wrangler development, such as
  `http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost`.
- Adds optional repeated `rel` filters to the target request.
- Shows HTTP status, content type, request URL, redirect `Location`, parsed JRD JSON, and raw
  response text.
- Renders JRD aliases, properties, and links in a readable layout.
- Provides a copyable `curl -i` command for the exact target lookup.

## Deploy

The Worker can be mounted below a path such as `/webfinger`. It still queries the standard target
path on the same host, such as `https://example.com/.well-known/webfinger`.

1. Install the local JavaScript tooling:

```console
npm install
```

1. Run a dry-run deploy:

```console
npm run deploy:dry-run
```

1. Deploy to the temporary Workers route:

```console
npm run deploy
```

1. Deploy-test on a site route without committing site-specific config:

```console
npx wrangler deploy \
  --route 'example.com/webfinger' \
  --route 'example.com/webfinger/*'
```

The route commands require the hostname's zone to be in the Cloudflare account and DNS to be set up
for that hostname. The checked-in `wrangler.toml` intentionally does not contain Josh-specific
routes or envs because this crate is a reusable tool, not a single-site deploy.

The deploy button is a convenience for Cloudflare's Git-backed flow. It is most useful after the
current branch is pushed to GitHub; the person deploying still needs to choose the account and add
any desired route such as `example.com/webfinger*` in Cloudflare.

## Local Development

Run Wrangler locally:

```console
npm run dev
```

Then open the local Wrangler URL and query a resource such as:

```text
http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost
```

That full URL shape lets a viewer on one local Wrangler port inspect a WebFinger responder running
on another local Wrangler port. Plain resource inputs are same-origin, so deployed pages should use
resources whose host matches the page host.

## Implementation Map

The Worker is split so future UI or protocol changes have one clear owner:

- `src/server.rs` owns Cloudflare Worker request handling. It routes the page shell and
  `/api/lookup`, rejects non-htmx or cross-site browser lookup requests, and returns htmx result
  fragments for the bundled form.
- `src/lookup.rs` owns WebFinger behavior. It parses viewer input, constructs the target
  `/.well-known/webfinger` URL, enforces same-origin or local-loopback policy, performs the
  server-side fetch, captures transport metadata, and returns the debugging payload.
- `src/view.rs` owns view models and renders Askama templates. It embeds `assets/app.css`,
  `assets/app.js`, and vendored `assets/vendor/htmx.min.js` into the page so deploys do not need
  separate asset routes and Cargo-only CI does not need npm packages installed. The vendored htmx
  file should match the pinned `htmx.org` version in the root `package.json` and
  `package-lock.json`.
- `templates/page.html` is the full page shell. `templates/lookup_result.html` and
  `templates/lookup_error.html` are htmx fragments swapped into `#results`.

htmx form submissions receive HTML fragments. Viewer-level failures such as malformed resources or
Worker fetch errors intentionally return `200` for htmx requests so the browser swaps the error
panel. `/api/lookup` is not a public JSON API: non-htmx callers receive `404`, and browser requests
with `Sec-Fetch-Site: cross-site` receive `403`. Those checks are defense in depth rather than
authentication; direct clients can spoof htmx and Fetch Metadata headers. They keep the supported
contract narrow and avoid CORS-enabled use of the Worker as a general server-side lookup endpoint.

Lookup input is bounded before the Worker fetches a target: resource strings, relation filter
count, relation filter length, final target URL length, same-origin target policy, and captured
response body size all have explicit limits. Response headers also set a restrictive baseline for
browser use, including
`Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, and `Permissions-Policy`.
The CSP allows inline script and style because the Worker embeds htmx, CSS, and local JavaScript in
one path-mounted page response; it still blocks remote scripts, framing, base URI changes, and
cross-origin connections.

The page is served as one HTML response even though the source is split into templates, CSS, and
JavaScript. That keeps path-mounted deploys such as `/webfinger` simple and avoids asset routing
and cache invalidation rules in this small Worker.

## Validation

Useful checks for this crate:

```console
cargo fmt --all --check
cargo test -p webfinger-viewer-worker
cargo check -p webfinger-viewer-worker --target wasm32-unknown-unknown
markdownlint-cli2 --no-globs webfinger-viewer-worker/README.md
npm run deploy:dry-run
```

`npm run deploy:dry-run` runs `worker-build --release` through the `wrangler.toml` build hook.

[deploy-button]: https://deploy.workers.cloudflare.com/button
[deploy-url]: https://deploy.workers.cloudflare.com/?url=https://github.com/joshka/webfinger-rs
