# Webfinger-rs

A rust crate implementing the Webfinger protocol ([RFC
7033](https://www.rfc-editor.org/rfc/rfc7033.html))

## Motivation

Existing crates have not been updated in some time and have a license that makes them difficult to
use within other products (GPL-3.0). They are also transport library specific. In contrast this
library is MIT / Apache licensed, and will be agnostic to the choice of request / server library as
it builds on the types in the [http](https://crates.io/crates/http) crate with conversions and
helper methods to make this easy.

## Features / TODO list

- [x] Client side types
- [x] Reqwest interaction
- [x] Server side types
- [x] Axum integration
- [ ] Actix integration

## Usage

```shell
cargo add webfinger-rs
```

```rust
let request = webfinger::Request {
    host: "example.com".parse()?,
    resource: "acct:carol@example.com",
    rel: "http://openid.net/specs/connect/1.0/issuer",
};
let response = request.fetch().await?;
```

The library also has a [cli](https://crates.io/crates/webfinger-cli) that can be useful to test
WebFinger servers.

```shell
cargo install webfinger-cli
webfinger fetch acct:carol@example.com example.com --rel http://openid.net/specs/connect/1.0/issuer
```

## Stability

This library is in early days and will have semver breaking changes in the 0.0.x releases. Once
0.1.0 is released, semver breaking changes will bump the minor version.

## License

Copyright (c) 2024 Josh McKinney

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
