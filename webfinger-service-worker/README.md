# WebFinger Service Worker

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/joshka/webfinger-rs)

`webfinger-service-worker` is a Rust Cloudflare Worker for serving WebFinger responses from
configuration stored in Workers KV. It is meant for people who want a deployable WebFinger endpoint
without writing their own server.

## Deploy

1. Click **Deploy to Cloudflare**.
1. Choose a Worker name and complete the deploy flow.
1. Open the created Workers KV namespace in the Cloudflare dashboard.
1. Add a key named `webfinger.toml`.
1. Paste the contents of [`webfinger.example.toml`](../webfinger-service/webfinger.example.toml) and
   replace the example values.
1. Add a Worker route for `example.com/.well-known/webfinger*`.
1. Verify the endpoint:

```console
curl 'https://example.com/.well-known/webfinger?resource=acct:alice@example.com'
```

The Worker reads KV binding `WEBFINGER_CONFIG` and key `webfinger.toml`.

If the KV key is missing, the Worker returns a setup message instead of serving the bundled example
identity. The example file is only a starting point for the value you paste into KV.

## Configuration

```toml
[[resources]]
resource = "acct:alice@example.com"
aliases = [
  "https://social.example/@alice",
  "https://social.example/users/alice",
]

[[resources.links]]
rel = "self"
type = "application/activity+json"
href = "https://social.example/users/alice"

[[resources.links]]
rel = "http://webfinger.net/rel/profile-page"
type = "text/html"
href = "https://social.example/@alice"

[[resources.links]]
rel = "http://ostatus.org/schema/1.0/subscribe"
template = "https://social.example/authorize_interaction?uri={uri}"
```

The lookup key is the exact `resource` string. When a request includes `rel` parameters, the Worker
returns only matching links.

String-valued JRD properties can be written as normal TOML strings. To publish a JSON `null`
property value, use `{ null = true }`.

## Local Development

Install dependencies and build the Worker:

```console
npm install
npm run deploy:dry-run
```

The checked-in `wrangler.toml` build hook runs `scripts/build-webfinger-service-worker.sh`, which
installs a minimal Rust toolchain only when `cargo` is missing, adds the Wasm target, and runs
`worker-build`. Local developers with Rust already installed use their existing toolchain.

Run locally:

```console
npm run dev
```

Seed KV from the CLI instead of the dashboard:

```console
npx wrangler kv key put webfinger.toml --binding WEBFINGER_CONFIG \
  --path webfinger-service/webfinger.example.toml
```

Deploy after updating `wrangler.toml` or configuring the route in the Cloudflare dashboard:

```console
npm run deploy
```

## Observability

`wrangler.toml` enables Cloudflare Worker observability. The Worker installs a console-backed
`tracing` subscriber in wasm builds, so request decisions and provider failures appear in Wrangler
tail and Cloudflare dashboard logs. Useful log events include:

- `webfinger service request` with `method`, `path`, and a stable `outcome` value.
- `resolved webfinger response` with the requested resource in the current tracing span.
- KV binding, read, and configuration errors at error level.

## Rust Extension Point

Applications that need D1, remote fetch, Durable Objects, or another async source can implement
`WebFingerProvider` and pass it to `Worker::new(provider).serve(request)` or
`serve_with_provider(&provider, request)`. The Worker crate owns Cloudflare HTTP response mapping
and wasm logging; `webfinger-service` owns the shared config and provider contracts.
