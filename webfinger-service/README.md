# WebFinger Service

`webfinger-service` is the runtime-neutral WebFinger responder core. It owns configuration parsing,
provider traits, exact resource matching, and relation filtering.

Runtime adapters live in separate crates:

- `webfinger-service-axum` provides the native Axum server.
- `webfinger-service-worker` provides the Cloudflare Worker.

## Configuration

The default empty config is [`webfinger.toml`](webfinger.toml):

```toml
resources = []
```

Use [`webfinger.example.toml`](webfinger.example.toml) as a starting point for a real responder:

```toml
[[resources]]
resource = "acct:alice@example.com"

[[resources.links]]
rel = "self"
type = "application/activity+json"
href = "https://social.example/users/alice"
```

The lookup key is the exact `resource` string. If a request includes repeated `rel` parameters, the
runtime adapters return only matching links.

Supported TOML fields map directly to JRD fields:

- resource-level: `resource`, `aliases`, `properties`.
- link-level: `rel`, `type`, `href`, `template`, `titles`, `properties`.

String-valued JRD properties can be written as normal TOML strings. To publish a JSON `null`
property value, use `{ null = true }`.

## Rust API

Use `StaticConfigProvider` when the configuration is already loaded into memory:

```rust
use webfinger_rs::WebFingerRequest;
use webfinger_service::{StaticConfigProvider, WebFingerProvider};

async fn resolve_alice() -> Result<(), Box<dyn std::error::Error>> {
    let provider = StaticConfigProvider::from_toml(
        r#"
[[resources]]
resource = "acct:alice@example.com"

[[resources.links]]
rel = "self"
type = "application/activity+json"
href = "https://social.example/users/alice"
"#,
    )?;
    let request = WebFingerRequest::builder("acct:alice@example.com")?
        .host("example.com")
        .build();

    let response = provider.resolve(&request).await?.unwrap();

    assert_eq!(response.subject.as_ref(), "acct:alice@example.com");
    Ok(())
}
```

## Extension Point

`WebFingerProvider` is the async boundary between runtime adapters and responder data. Implement it
when responses come from a database, Workers KV, D1, a remote fetch, Durable Objects, or another
source that cannot be loaded into `StaticConfigProvider`.

A provider receives a parsed `WebFingerRequest` and returns one domain result:

- `Ok(Some(response))` when the requested resource is known.
- `Ok(None)` when the request is valid but the resource is unknown.
- `Err(error)` when configuration, storage, or provider logic failed.

Providers own exact resource lookup and `rel` filtering. If `request.rels` is not empty, return only
links whose `rel` is present in that list. Runtime adapters own HTTP status codes, logging, and
response headers.

```rust
use std::collections::BTreeMap;

use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse};
use webfinger_service::{ProviderError, WebFingerProvider};

#[derive(Default)]
struct DirectoryProvider {
    resources: BTreeMap<String, WebFingerResponse>,
}

impl WebFingerProvider for DirectoryProvider {
    async fn resolve<'a>(
        &'a self,
        request: &'a WebFingerRequest,
    ) -> Result<Option<WebFingerResponse>, ProviderError> {
        let Some(response) = self.resources.get(request.resource.as_ref()) else {
            return Ok(None);
        };

        let mut response = response.clone();
        if !request.rels.is_empty() {
            response.links.retain(|link| request.rels.contains(&link.rel));
        }
        Ok(Some(response))
    }
}

async fn lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = DirectoryProvider::default();
    let response = WebFingerResponse::try_builder("acct:alice@example.com")?
        .link(Link::builder(Rel::new("self")).href("https://social.example/users/alice"))
        .build();
    provider
        .resources
        .insert(response.subject.to_string(), response);

    let request = WebFingerRequest::builder("acct:alice@example.com")?
        .host("example.com")
        .rel("self")
        .build();

    let response = provider.resolve(&request).await?.unwrap();

    assert_eq!(response.links.len(), 1);
    Ok(())
}
```
