# Webfinger-rs

[![Crates.io badge]][crate]
[![License badge]][license]
[![Docs.rs badge]][docs]
[![Deps.rs badge]][dependencies]

<!-- cargo-rdme start -->

`webfinger-rs` is a Rust library for handling WebFinger protocol defined by [RFC 7033].

WebFinger is  is used to discover information about people or other entities on the internet.
The motivation of this library is to provide a transport-agnostic implementation of the
WebFinger protocol for client and server-side application which can be used with different HTTP
libraries such as [Axum], and [Reqwest]. Additionally, the other available crates for WebFinger
are either not actively maintained and have a license that is incompatible with incorporating
the crate into other projects as a library (GPL-3.0).

[RFC 7033]: https://www.rfc-editor.org/rfc/rfc7033.html
[Axum]: https://crates.io/crates/axum
[Reqwest]: https://crates.io/crates/reqwest

## Usage

To use this library, add it to your `Cargo.toml`:

```shell
cargo add webfinger-rs
```

The library also has a related CLI tool, `webfinger-cli`, which can be installed with:

```shell
cargo install webfinger-cli
webfinger fetch acct:carol@example.com --rel http://webfinger.net/rel/avatar
```

## Client Example

The following example connects to the WebFinger server at `example.com` and requests the profile
page for the user `carol@example.com`. It requires the `reqwest` feature to be enabled. This
example is also available in the repository at:
<https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/client.rs>.

```rust, no_run
use webfinger_rs::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::builder("acct:carol@example.com")?
        .host("example.com")
        .rel("http://webfinger.net/rel/profile-page")
        .build();
    let response = request.execute().await?;
    dbg!(response);
    Ok(())
}
```

## Server Example

The following example is an Axum handler that responds to WebFinger requests. It requires the
`axum` feature to be enabled. This example is also available in the repository at:
<https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs>.

```rust
use axum::response::Result as AxumResult;
use webfinger_rs::{Link, Rel, Request as WebFingerRequest, Response as WebFingerResponse};

async fn webfinger(request: WebFingerRequest) -> AxumResult<WebFingerResponse> {
    let subject = request.resource.to_string();
    if subject != "acct:carol@example.com" {
        Err((http::StatusCode::NOT_FOUND, "Not Found"))?;
    }
    let rel = Rel::new("http://webfinger.net/rel/profile-page");
    let response = if request.rels.is_empty() || request.rels.contains(&rel) {
        let link = Link::builder(rel).href(format!("https://example.com/profile/{subject}"));
        WebFingerResponse::builder(subject).link(link).build()
    } else {
        WebFingerResponse::builder(subject).build()
    };
    Ok(response)
}
```

## Features / TODO list

- [x] Client side types
- [x] Reqwest interaction
- [x] Server side types
- [x] Axum integration
- [ ] Actix integration

## Stability

This library is in early days and will have semver breaking changes in the 0.0.x releases. Once
0.1.0 is released, semver breaking changes will bump the minor version.

### License

Copyright (c) 2024 Josh McKinney

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
  at your option.

<!-- cargo-rdme end -->

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).

[Crates.io badge]: https://img.shields.io/crates/v/webfinger-rs?logo=rust&style=for-the-badge
[License badge]: https://img.shields.io/crates/l/webfinger-rs?style=for-the-badge
[Docs.rs badge]: https://img.shields.io/docsrs/webfinger-rs?logo=rust&style=for-the-badge
[Deps.rs badge]: https://deps.rs/repo/github/joshka/webfinger-rs/status.svg?style=for-the-badge
[crate]: https://crates.io/crates/webfinger-rs
[license]: ./LICENSE-MIT
[docs]: https://docs.rs/webfinger-rs/
[dependencies]: https://deps.rs/repo/github/joshka/webfinger-rs
