# Webfinger-rs

[![Crates.io badge]][crate]
[![License badge]][license]
[![Docs.rs badge]][docs]
[![Deps.rs badge]][dependencies]

`webfinger-rs` is a Rust library for building and serving [WebFinger] requests and responses with
[RFC 7033]-shaped types and first-party integrations for [Reqwest], [Axum], and [Actix Web].

The crate keeps request parsing, JRD response construction, and framework adapters in one place so
clients, servers, and tests use the same WebFinger types.

The docs.rs page is the full API reference and usage guide.

## Why `webfinger-rs`

- Model WebFinger requests and JRD responses with reusable library types.
- Execute client requests with Reqwest.
- Expose WebFinger endpoints in Axum or Actix Web with the same request and response types.
- Stay close to [RFC 7033] without pulling in a larger identity stack.

## Supported integrations

| Feature | What it enables |
| --- | --- |
| none | Core request and response types, builders, and URL conversion |
| `reqwest` | Client execution helpers and Reqwest request/response conversions |
| `axum` | Axum extractor and responder integration |
| `actix` | Actix Web extractor and responder integration |

Current integration targets:

- Reqwest `0.13`
- Axum `0.8`
- Actix Web `4`

## Repository tools

This repository also contains a configurable WebFinger responder split across three crates:

- [`webfinger-service`](webfinger-service/README.md) is the runtime-neutral core. It parses TOML
  configuration, owns provider traits, and resolves exact WebFinger resources with `rel` filtering.
- [`webfinger-service-axum`](webfinger-service-axum/README.md) is the native Axum server for local
  development and non-Worker deployments.
- [`webfinger-service-worker`](webfinger-service-worker/README.md) is the Cloudflare Worker
  adapter. It reads TOML configuration from Workers KV and serves `/.well-known/webfinger`.

This repository also contains `webfinger-viewer-worker`, a separate Rust Cloudflare Worker that
serves a browser UI for inspecting WebFinger discovery behavior. It is a debugging tool, not the
server-side WebFinger responder: the page accepts an `acct:` resource or full WebFinger URL, asks
the Worker to fetch the target `/.well-known/webfinger` endpoint server-side, and renders the
target HTTP status, content type, redirect `Location`, parsed JRD fields, raw JSON, and a copyable
`curl` command.

The viewer is same-origin by default for public deployments, so a page mounted at
`https://example.com/webfinger` inspects `https://example.com/.well-known/webfinger`. Local
Wrangler sessions can use full loopback WebFinger URLs to inspect another local server. See
[`webfinger-viewer-worker/README.md`](webfinger-viewer-worker/README.md) for local development,
validation, and deployment notes.

## Primary types

- `WebFingerRequest` models the WebFinger query target, host, and optional relation filters. Build
  one directly for client requests, or extract one from an Axum or Actix handler.
- `WebFingerResponse` models the JSON Resource Descriptor returned by a WebFinger endpoint. Return
  one from server handlers or parse one from a Reqwest response.
- `Link` and `Rel` model JRD link objects and relation filters so servers can apply the same
  relation-filtering rules that clients request.
- `Resource` and `JrdUri` validate URI-valued protocol fields before they enter requests or JRD
  responses.

## Protocol overview

A WebFinger query is an HTTPS `GET` against `/.well-known/webfinger` with a required `resource`
parameter and, optionally, one or more `rel` parameters. The `resource` parameter is the query
target URI; builders and server extractors reject relative references such as `carol`,
`/relative`, `../x`, and empty values.

A request built by this crate today for `acct:carol@example.com` filtered to the profile-page
relation looks like this:

```text
GET https://example.com/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fwebfinger.net%2Frel%2Fprofile-page
```

Server integrations leave routing and TLS at the framework boundary, then use WebFinger extractors
for protocol parsing:

- mount the handler as `GET` at `/.well-known/webfinger` so the router rejects other paths and
  methods;
- configure TLS and forwarded-proto handling at the server or reverse-proxy boundary; and
- let the Axum or Actix Web extractor validate the request host, query parameters, percent
  encoding, and `resource` URI.

## Install

Add the crate with the feature set you need:

```shell
cargo add webfinger-rs
cargo add webfinger-rs --features reqwest
cargo add webfinger-rs --features axum
cargo add webfinger-rs --features actix
```

The companion CLI is useful for trying servers by hand:

```shell
cargo install webfinger-cli
webfinger acct:carol@example.com --rel http://webfinger.net/rel/avatar
```

## Axum server quick example

Enable `axum` to extract `WebFingerRequest` and return `WebFingerResponse` from a handler mounted
at `/.well-known/webfinger`:

```shell
cargo add webfinger-rs --features axum
cargo add axum
```

```rust
use axum::{http::StatusCode, routing::get, Router};
use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};

async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
    let subject = request.resource.to_string();
    if subject != "acct:carol@example.com" {
        return Err((StatusCode::NOT_FOUND, "not found").into());
    }

    let rel = Rel::new("http://webfinger.net/rel/profile-page");
    let response = if request.rels.is_empty() || request.rels.contains(&rel) {
        let link = Link::builder(rel).href("https://example.com/users/carol");
        WebFingerResponse::builder(subject).link(link).build()
    } else {
        WebFingerResponse::builder(subject).build()
    };
    Ok(response)
}

Router::new().route(WELL_KNOWN_PATH, get(webfinger))
```

## Reqwest client quick example

Enable `reqwest` to execute a request directly from `WebFingerRequest`:

```rust
use webfinger_rs::WebFingerRequest;

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let request = WebFingerRequest::builder("acct:carol@example.com")?
        .host("example.com")
        .rel("http://webfinger.net/rel/profile-page")
        .build();

    let response = request.execute_reqwest().await?;
    println!("{response}");
    Ok(())
}
```

## Learn more

- API docs and deeper usage guide: [docs.rs/webfinger-rs][docs]
- Runnable example servers:
  `cargo run -p webfinger-rs --example axum --features axum`
  `cargo run -p webfinger-rs --example actix --features actix`
- Deployable WebFinger service Worker:
  [`webfinger-service-worker`](webfinger-service-worker/README.md)
- Configurable native WebFinger service:
  [`webfinger-service-axum`](webfinger-service-axum/README.md)
- Runtime-neutral WebFinger service core:
  [`webfinger-service`](webfinger-service/README.md)
- Runnable example client:
  `cargo run -p webfinger-rs --example client --features reqwest`
- CLI crate: [`webfinger-cli`](https://crates.io/crates/webfinger-cli)

Run one server example first, then run the client example in another shell. The client example
queries `https://localhost:3000`, accepts the self-signed certificate generated by either server
example, and prints the shared `WebFingerResponse` as JSON.

The server examples also work with the CLI. Query without `--rel` to get both links, or pass a
relation filter to narrow the returned `links` array:

```shell
webfinger acct:carol@localhost localhost:3000 --insecure
webfinger acct:carol@localhost localhost:3000 --insecure \
  --rel http://webfinger.net/rel/profile-page
webfinger acct:carol@localhost localhost:3000 --insecure \
  --rel http://webfinger.net/rel/avatar
```

## License

Copyright (c) Josh McKinney

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>) at your
  option

## MSRV

This library is tested on the latest stable release of Rust. The minimum supported version is the
previous stable release. The library may work on older versions, but that is not guaranteed.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).

[WebFinger]: https://en.wikipedia.org/wiki/WebFinger
[RFC 7033]: https://www.rfc-editor.org/rfc/rfc7033.html
[Reqwest]: https://crates.io/crates/reqwest
[Axum]: https://crates.io/crates/axum
[Actix Web]: https://crates.io/crates/actix-web
[Crates.io badge]: https://img.shields.io/crates/v/webfinger-rs?logo=rust&style=for-the-badge
[License badge]: https://img.shields.io/crates/l/webfinger-rs?style=for-the-badge
[Docs.rs badge]: https://img.shields.io/docsrs/webfinger-rs?logo=rust&style=for-the-badge
[Deps.rs badge]: https://deps.rs/repo/github/joshka/webfinger-rs/status.svg?style=for-the-badge
[crate]: https://crates.io/crates/webfinger-rs
[license]: ./LICENSE-MIT
[docs]: https://docs.rs/webfinger-rs/
[dependencies]: https://deps.rs/repo/github/joshka/webfinger-rs
