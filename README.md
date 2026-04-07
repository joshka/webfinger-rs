# Webfinger-rs

[![Crates.io badge]][crate]
[![License badge]][license]
[![Docs.rs badge]][docs]
[![Deps.rs badge]][dependencies]

`webfinger-rs` is a Rust library for building and serving [WebFinger] requests and responses with
[RFC 7033]-shaped types and first-party integrations for [Reqwest], [Axum], and [Actix Web].

Use it when you want one WebFinger implementation across clients, servers, and tests instead of
recreating the protocol details in each framework.

The full guide and API reference live on [docs.rs][docs].

## Why `webfinger-rs`

- Model WebFinger requests and JRD responses with reusable library types.
- Execute client requests with Reqwest.
- Expose WebFinger endpoints in Axum or Actix Web with the same request and response types.
- Stay close to [RFC 7033] without pulling in a larger identity stack.

## Supported integrations

| Feature | What it enables |
| --- | --- |
| none | Core request and response types, builders, and URL conversion support |
| `reqwest` | Client execution helpers and Reqwest request/response conversions |
| `axum` | Axum extractor and responder integration |
| `actix` | Actix Web extractor and responder integration |

Current integration targets:

- Reqwest `0.13`
- Axum `0.8`
- Actix Web `4`

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

## Client quickstart

Enable `reqwest` to execute a request directly from `WebFingerRequest`:

```rust
use webfinger_rs::WebFingerRequest;

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let request = WebFingerRequest::builder("acct:carol@example.com")?
        .host("example.com")
        .rel("http://webfinger.net/rel/profile-page")
        .build();

    let response = request.execute_reqwest().await?;
    println!("{response:#?}");
    Ok(())
}
```

## Server quickstart

Enable `axum` to extract `WebFingerRequest` and return `WebFingerResponse` from a handler mounted
at `/.well-known/webfinger`:

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

Enable `actix` to use the same types in Actix Web:

```rust
use actix_web::{get, App};
use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse};

#[get("/.well-known/webfinger")]
async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
    let subject = request.resource.to_string();
    if subject != "acct:carol@example.com" {
        return Err(actix_web::error::ErrorNotFound("not found"));
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

App::new().service(webfinger)
```

## Learn more

- API docs and deeper usage guide: [docs.rs/webfinger-rs][docs]
- Runnable examples:
  `cargo run -p webfinger-rs --example client --features reqwest`
  `cargo run -p webfinger-rs --example axum --features axum`
  `cargo run -p webfinger-rs --example actix --features actix`
- CLI crate: [`webfinger-cli`](https://crates.io/crates/webfinger-cli)

The example servers listen on `https://localhost:3000` and can be queried with:

```shell
webfinger acct:carol@localhost localhost:3000 --insecure \
  --rel http://webfinger.net/rel/profile-page
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
