//! Runtime-neutral viewer configuration.
//!
//! Runtime adapters load configuration from their environment and pass this typed model into the
//! shared viewer. Bind settings are useful to native adapters, while lookup settings affect shared
//! request construction and policy.

use serde::Deserialize;
use thiserror::Error;

/// WebFinger viewer configuration loaded from TOML.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewerConfig {
    /// Native server defaults used by runtime adapters that bind a local socket.
    #[serde(default)]
    pub server: ServerConfig,

    /// Runtime-neutral lookup behavior.
    #[serde(default)]
    pub lookup: LookupConfig,
}

impl ViewerConfig {
    /// Parses viewer configuration from TOML.
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(input)?)
    }
}

/// Native server bind defaults.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Host address to bind.
    pub host: Option<String>,

    /// TCP port to bind.
    pub port: Option<u16>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Some("127.0.0.1".to_string()),
            port: Some(8788),
        }
    }
}

/// Shared lookup behavior.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupConfig {
    /// Local responder port used for loopback `acct:` resources during local development.
    pub local_responder_port: u16,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            local_responder_port: 8787,
        }
    }
}

/// Errors raised while parsing viewer TOML configuration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The TOML was malformed.
    #[error("invalid TOML configuration: {0}")]
    Toml(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_config_with_defaults() {
        let config = ViewerConfig::from_toml("").unwrap();

        assert_eq!(config.server.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(config.server.port, Some(8788));
        assert_eq!(config.lookup.local_responder_port, 8787);
    }

    #[test]
    fn parses_lookup_port_override() {
        let config = ViewerConfig::from_toml(
            r#"
            [lookup]
            local_responder_port = 8790
            "#,
        )
        .unwrap();

        assert_eq!(config.lookup.local_responder_port, 8790);
    }

    #[test]
    fn rejects_unknown_config_fields() {
        let error = ViewerConfig::from_toml(
            r#"
            [lookup]
            local_responder_port = 8790
            timeout_seconds = 5
            "#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("timeout_seconds"));
    }
}
