use super::*;
use webfinger_rs::WebFingerRequest;

const CONFIG: &str = r#"
[[resources]]
resource = "acct:alice@example.com"
aliases = ["https://social.example/@alice"]

[resources.properties]
"https://example.com/ns/display-name" = "Alice"
"https://example.com/ns/old-name" = { null = true }

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
"#;

#[test]
fn parses_valid_toml_config() {
    let config = Config::from_toml(CONFIG).unwrap();

    let request = request("acct:alice@example.com", []);
    let response = config.resolve(&request).unwrap();

    assert_eq!(response.subject.as_ref(), "acct:alice@example.com");
    assert_eq!(response.links.len(), 3);
    assert_eq!(
        response
            .properties
            .unwrap()
            .get("https://example.com/ns/old-name"),
        Some(&None),
    );
    assert_eq!(
        response.links[2].template.as_deref(),
        Some("https://social.example/authorize_interaction?uri={uri}"),
    );
}

#[test]
fn rejects_malformed_toml_config() {
    let error = Config::from_toml("[[resources]").unwrap_err();

    assert!(matches!(error, ConfigError::Toml(_)));
}

#[test]
fn rejects_duplicate_resource_entries() {
    let error = Config::from_toml(
        r#"
[[resources]]
resource = "acct:alice@example.com"

[[resources]]
resource = "acct:alice@example.com"
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        ConfigError::DuplicateResource(resource)
            if resource == "acct:alice@example.com"
    ));
}

#[test]
fn rejects_false_resource_property_null_marker() {
    let error = Config::from_toml(
        r#"
[[resources]]
resource = "acct:alice@example.com"

[resources.properties]
"https://example.com/ns/display-name" = { null = false }
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        ConfigError::InvalidNullProperty(property)
            if property == "https://example.com/ns/display-name"
    ));
}

#[test]
fn rejects_false_link_property_null_marker() {
    let error = Config::from_toml(
        r#"
[[resources]]
resource = "acct:alice@example.com"

[[resources.links]]
rel = "self"

[resources.links.properties]
"https://example.com/ns/display-name" = { null = false }
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        ConfigError::InvalidNullProperty(property)
            if property == "https://example.com/ns/display-name"
    ));
}

#[test]
fn parses_empty_default_config() {
    let config = Config::from_toml(include_str!("../webfinger.toml")).unwrap();

    let request = request("acct:alice@example.com", []);

    assert!(config.resolve(&request).is_none());
}

#[test]
fn resolves_exact_resource_matches() {
    let config = Config::from_toml(EXAMPLE_CONFIG).unwrap();

    let request = request("acct:alice@example.com", []);

    assert!(config.resolve(&request).is_some());
}

#[test]
fn returns_none_for_unknown_resources() {
    let config = Config::from_toml(EXAMPLE_CONFIG).unwrap();

    let request = request("acct:bob@example.com", []);

    assert!(config.resolve(&request).is_none());
}

#[test]
fn omits_relation_filter_when_no_rel_is_requested() {
    let config = Config::from_toml(EXAMPLE_CONFIG).unwrap();

    let request = request("acct:alice@example.com", []);
    let response = config.resolve(&request).unwrap();

    assert_eq!(response.links.len(), 3);
}

#[test]
fn filters_repeated_relation_requests() {
    let config = Config::from_toml(EXAMPLE_CONFIG).unwrap();

    let request = request(
        "acct:alice@example.com",
        [
            "self",
            "http://webfinger.net/rel/profile-page",
            "http://webfinger.net/rel/avatar",
        ],
    );
    let response = config.resolve(&request).unwrap();

    assert_eq!(response.links.len(), 2);
}

fn request<const N: usize>(resource: &str, rels: [&str; N]) -> WebFingerRequest {
    let mut builder = WebFingerRequest::builder(resource)
        .unwrap()
        .host("example.com");
    for rel in rels {
        builder = builder.rel(rel);
    }
    builder.build()
}
