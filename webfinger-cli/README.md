# Webfinger-cli

[![Crates.io badge]][crate]
[![License badge]][license]
[![Deps.rs badge]][dependencies]

`webfinger-cli` is a command line tool for querying WebFinger servers. It is built on top of the
[`webfinger-rs`] library, which provides a transport-agnostic implementation of the WebFinger
protocol defined by [RFC 7033].

[`webfinger-rs`]: https://crates.io/crates/webfinger-rs
[RFC 7033]: https://www.rfc-editor.org/rfc/rfc7033.html

## Installation

To install the `webfinger-cli`, you can use the following command:

```shell
cargo install webfinger-cli
```

## Usage

```plain
Usage: webfinger [OPTIONS] <RESOURCE> [HOST]

Arguments:
  <RESOURCE>  The resource to fetch
  [HOST]      The host to fetch the webfinger resource from

Options:
  -r, --rel <REL>   The link relation types to fetch
      --insecure    Ignore TLS certificate verification errors
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help (see more with '--help')
```

E.g. to get the avatar for a user with the account `carol@example.com`, you can run:

```shell
webfinger acct:carol@example.com --rel http://webfinger.net/rel/avatar
```

![Made with VHS](https://vhs.charm.sh/vhs-1oNVS5B2maoeyAxFCHYpSz.gif)

## License

Copyright (c) Josh McKinney

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>) at your
  option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).

[Crates.io badge]: https://img.shields.io/crates/v/webfinger-cli?logo=rust&style=for-the-badge
[License badge]: https://img.shields.io/crates/l/webfinger-cli?style=for-the-badge
[Deps.rs badge]: https://deps.rs/repo/github/joshka/webfinger-rs/status.svg?style=for-the-badge
[crate]: https://crates.io/crates/webfinger-cli
[license]: ./LICENSE-MIT
[dependencies]: https://deps.rs/repo/github/joshka/webfinger-rs
