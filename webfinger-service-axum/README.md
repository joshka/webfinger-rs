# WebFinger Service Axum

`webfinger-service-axum` is the native Axum server for `webfinger-service`. It owns Axum request
extraction, response mapping, Tower HTTP tracing, and the local development binary.

## Run Locally

```console
cargo run -p webfinger-service-axum
```

By default this reads `webfinger-service/webfinger.toml`, an empty config with no WebFinger
responses, and listens on `127.0.0.1:8788`. Lookups return `404` until you point the server at a
config file with resources.

```console
cargo run -p webfinger-service-axum -- --example-config --port 8790
curl 'http://127.0.0.1:8790/.well-known/webfinger?resource=acct:alice@example.com'
```

The CLI also accepts environment variables:

```console
WEBFINGER_CONFIG_FILE=webfinger-service/webfinger.example.toml \
  PORT=8790 \
  cargo run -p webfinger-service-axum
```

Useful options:

- `--config <PATH>` or `WEBFINGER_CONFIG_FILE` chooses a TOML config file.
- `--example-config` serves `webfinger-service/webfinger.example.toml`.
- `--host <HOST>` or `HOST` chooses the bind host.
- `--port <PORT>` or `PORT` chooses the bind port.

If the selected config path cannot be read, the process exits with a message that names the path. If
the bind address is already in use, the process names the address and suggests changing `PORT` or
stopping the process that owns the port.

## Observability

The Axum runtime installs compact `tracing-subscriber` output and a Tower HTTP trace layer. A plain
`cargo run -p webfinger-service-axum` prints the bind address, and it prints request and response
lines when the log filter allows `info` events. Set `RUST_LOG` to tune native logs:

```console
RUST_LOG=webfinger_service_axum=debug,tower_http=debug cargo run -p webfinger-service-axum
```

Useful log events include:

- `webfinger service request` with `method`, `path`, and a stable `outcome` value.
- provider and configuration errors at error level.

When `RUST_LOG` is unset, the server defaults to `info`. Set `RUST_LOG` to any standard
`tracing-subscriber` filter to tune or silence logs.
