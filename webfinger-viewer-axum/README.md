# WebFinger Viewer Axum

`webfinger-viewer-axum` runs the shared WebFinger viewer as a standard Axum application. It owns
native request extraction, response construction, reqwest-backed target fetches, rustls setup,
Tokio startup, Tower HTTP tracing, and native logging.

The shared route policy, lookup validation, htmx response behavior, Askama templates, and assets
live in [`webfinger-viewer`](../webfinger-viewer/README.md).

## Local Development

Run the viewer as a native Axum server:

```console
cargo run -p webfinger-viewer-axum
```

Then open `http://127.0.0.1:8788/webfinger`. Set `PORT` to choose a different local port:

```console
PORT=8791 cargo run -p webfinger-viewer-axum
```

By default this reads `webfinger-viewer/viewer.toml`. The CLI also accepts explicit flags:

```console
cargo run -p webfinger-viewer-axum -- --host 127.0.0.1 --port 8791
cargo run -p webfinger-viewer-axum -- --config webfinger-viewer/viewer.example.toml
cargo run -p webfinger-viewer-axum -- --example-config
```

The same values can be supplied through environment variables:

```console
WEBFINGER_VIEWER_CONFIG_FILE=webfinger-viewer/viewer.example.toml \
  HOST=127.0.0.1 \
  PORT=8791 \
  cargo run -p webfinger-viewer-axum
```

Useful options:

- `--config <PATH>` or `WEBFINGER_VIEWER_CONFIG_FILE` chooses a TOML config file.
- `--example-config` uses `webfinger-viewer/viewer.example.toml`.
- `--host <HOST>` or `HOST` chooses the bind host.
- `--port <PORT>` or `PORT` chooses the bind port.

Config file values are the base defaults; environment variables and CLI flags override bind
settings. The shared `[lookup]` settings still come from the selected config file.

Default config:

```toml
[server]
host = "127.0.0.1"
port = 8788

[lookup]
local_responder_port = 8787
```

The bundled example config changes both the default bind port and the local responder port:

```toml
[server]
host = "127.0.0.1"
port = 8791

[lookup]
local_responder_port = 8790
```

Open the local URL and query a resource such as:

```text
acct:joshka@hachyderm.io
```

To inspect a local WebFinger responder running on another port, query the full target URL:

```text
http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost
```

For the common local responder port, plain loopback resources also work:

```text
acct:alice@localhost
acct:alice@127.0.0.1
```

If you type the standard HTTPS form for a loopback target, local mode normalizes it to HTTP before
fetching because local debugging servers use plain HTTP:

```text
https://localhost:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost
```

Production deployments should use resources whose host matches the page host. Local sessions are
intentionally more permissive because they are a developer debugging surface.

## Observability

The Axum runtime installs compact `tracing-subscriber` output and a Tower HTTP trace layer. A plain
`cargo run -p webfinger-viewer-axum` prints the bind address, and it prints request and response
lines when the log filter allows `info` events. Set `RUST_LOG` to tune native logs:

```console
RUST_LOG=webfinger_viewer=debug,webfinger_viewer_axum=debug,tower_http=debug cargo run -p webfinger-viewer-axum
```

Useful log events include:

- `webfinger viewer request` with `method`, `path`, and a stable `outcome` value.
- `webfinger lookup result` with target status, target URL, resource, content type, and truncation
  state.
- lookup input and native fetch errors at error level.

## Implementation Map

- `src/main.rs` owns Tokio startup, rustls provider setup, reqwest client construction, bind
  address parsing, config loading, Clap argument parsing, and native tracing setup.
- `src/axum.rs` owns Axum request handling, native response construction, form parsing, Tower HTTP
  tracing, and the reqwest-backed target fetch path.
- [`webfinger-viewer`](../webfinger-viewer/README.md) owns the shared route policy, lookup
  validation, htmx fragments, templates, and assets.

## Validation

Useful checks for this crate:

```console
cargo fmt --all --check
cargo test -p webfinger-viewer-axum
cargo run -p webfinger-viewer-axum -- --port 8791
cargo run -p webfinger-viewer-axum -- --example-config --port 8792
markdownlint-cli2 --no-globs webfinger-viewer-axum/README.md
```
