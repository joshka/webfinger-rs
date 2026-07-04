//! Shared WebFinger responder runtime.
//!
//! This crate owns the runtime-neutral WebFinger responder behavior: TOML configuration parsing,
//! provider traits, and relation filtering. Native Axum support lives in
//! `webfinger-service-axum`. Cloudflare-specific KV and Worker entrypoints live in
//! `webfinger-service-worker`.
//!
//! # Example
//!
//! ```
//! use webfinger_rs::WebFingerRequest;
//! use webfinger_service::{StaticConfigProvider, WebFingerProvider};
//!
//! # async fn resolve_alice() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = StaticConfigProvider::from_toml(
//!     r#"
//! [[resources]]
//! resource = "acct:alice@example.com"
//!
//! [[resources.links]]
//! rel = "self"
//! type = "application/activity+json"
//! href = "https://social.example/users/alice"
//! "#,
//! )?;
//! let request = WebFingerRequest::builder("acct:alice@example.com")?
//!     .host("example.com")
//!     .build();
//!
//! let response = provider.resolve(&request).await?.unwrap();
//!
//! assert_eq!(response.subject.as_ref(), "acct:alice@example.com");
//! assert_eq!(response.links.len(), 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Provider Model
//!
//! [`WebFingerProvider`] is the async extension point used by runtime adapters. Implement it when
//! WebFinger responses come from a database, Workers KV, a remote service, or another source that
//! cannot be represented as static TOML. Providers own exact resource lookup and `rel` filtering;
//! adapters own HTTP status codes, response headers, and logging.

mod config;
mod provider;

#[cfg(test)]
mod tests;

pub use crate::config::{Config, ConfigError};
pub use crate::provider::{ProviderError, StaticConfigProvider, WebFingerProvider};

/// The default configuration key used by deployable runtimes.
pub const WEBFINGER_CONFIG_KEY: &str = "webfinger.toml";

/// An example configuration suitable for the first value a user edits.
pub const EXAMPLE_CONFIG: &str = include_str!("../webfinger.example.toml");
