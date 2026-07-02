use webfinger_rs::{WebFingerRequest, WebFingerResponse};
use webfinger_service::{ProviderError, WEBFINGER_CONFIG_KEY, WebFingerProvider};
use worker::Env;

/// The default Workers KV binding name for WebFinger configuration.
///
/// Binding names are local to a Worker. Different Cloudflare accounts can use the same binding
/// name while pointing at account-specific KV namespaces.
pub const WEBFINGER_CONFIG_BINDING: &str = "WEBFINGER_CONFIG";

/// A provider backed by TOML stored in Workers KV.
///
/// `KvConfigProvider` reads the configured KV key on each WebFinger lookup, parses the TOML into a
/// [`webfinger_service::Config`], and resolves the request against that config. This keeps the
/// Cloudflare dashboard KV editor as the source of truth: changing the `webfinger.toml` value
/// updates future requests without rebuilding the Worker.
///
/// Use [`KvConfigProvider::from_env`] for the conventional `WEBFINGER_CONFIG` binding and
/// `webfinger.toml` key. Use [`KvConfigProvider::from_env_binding`] when embedding this provider in
/// a Worker with different binding or key names.
#[derive(Clone)]
pub struct KvConfigProvider {
    kv: worker::kv::KvStore,
    key: String,
}

impl std::fmt::Debug for KvConfigProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvConfigProvider")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl KvConfigProvider {
    /// Creates a KV-backed provider from a Cloudflare Worker environment.
    ///
    /// This uses the documented `WEBFINGER_CONFIG` binding and `webfinger.toml` key.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Binding`] if the Worker environment does not contain the expected
    /// KV binding.
    pub fn from_env(env: &Env) -> Result<Self, ProviderError> {
        Self::from_env_binding(env, WEBFINGER_CONFIG_BINDING, WEBFINGER_CONFIG_KEY)
    }

    /// Creates a KV-backed provider from a named binding and key.
    ///
    /// This is useful for custom deployments that want to keep the provider behavior but use a
    /// different binding name or config key.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Binding`] if `binding` is not present in the Worker environment.
    pub fn from_env_binding(env: &Env, binding: &str, key: &str) -> Result<Self, ProviderError> {
        let kv = env.kv(binding).map_err(|source| ProviderError::Binding {
            binding: binding.to_string(),
            message: source.to_string(),
        })?;
        Ok(Self {
            kv,
            key: key.to_string(),
        })
    }
}

impl WebFingerProvider for KvConfigProvider {
    async fn resolve<'a>(
        &'a self,
        request: &'a WebFingerRequest,
    ) -> Result<Option<WebFingerResponse>, ProviderError> {
        let input = self
            .kv
            .get(&self.key)
            .text()
            .await
            .map_err(|source| ProviderError::ReadConfig {
                key: self.key.clone(),
                message: source.to_string(),
            })?
            .ok_or_else(|| ProviderError::MissingConfig {
                key: self.key.clone(),
            })?;
        let config = webfinger_service::Config::from_toml(&input)?;
        Ok(config.resolve(request))
    }
}
