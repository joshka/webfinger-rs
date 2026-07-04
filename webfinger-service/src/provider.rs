use std::future::Future;

use thiserror::Error;
use webfinger_rs::{WebFingerRequest, WebFingerResponse};

use crate::{Config, ConfigError};

/// Resolves a WebFinger request into an optional JRD response.
///
/// Providers are the data boundary for a WebFinger service. A provider may read from static
/// configuration, Workers KV, D1, a database, a remote fetch, a Durable Object, or any other source
/// that can answer a [`WebFingerRequest`].
///
/// The provider owns resource lookup and relation filtering:
///
/// - Return `Ok(Some(response))` when the requested `resource` is known.
/// - Return `Ok(None)` when the request is valid but the resource is unknown.
/// - Return `Err(error)` when the backing store, configuration, or provider logic failed.
/// - If `request.rels` is not empty, return only links whose `rel` is present in that list.
///
/// Runtime adapters such as `webfinger-service-axum` and `webfinger-service-worker` own HTTP status
/// codes, response headers, and logging. Providers should return domain results rather than HTTP
/// responses.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
///
/// use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse};
/// use webfinger_service::{ProviderError, WebFingerProvider};
///
/// #[derive(Default)]
/// struct DirectoryProvider {
///     resources: BTreeMap<String, WebFingerResponse>,
/// }
///
/// impl WebFingerProvider for DirectoryProvider {
///     async fn resolve<'a>(
///         &'a self,
///         request: &'a WebFingerRequest,
///     ) -> Result<Option<WebFingerResponse>, ProviderError> {
///         let Some(response) = self.resources.get(request.resource.as_ref()) else {
///             return Ok(None);
///         };
///
///         let mut response = response.clone();
///         if !request.rels.is_empty() {
///             response.links.retain(|link| request.rels.contains(&link.rel));
///         }
///         Ok(Some(response))
///     }
/// }
///
/// # async fn lookup() -> Result<(), Box<dyn std::error::Error>> {
/// let mut provider = DirectoryProvider::default();
/// let response = WebFingerResponse::try_builder("acct:alice@example.com")?
///     .link(Link::builder(Rel::new("self")).href("https://social.example/users/alice"))
///     .build();
/// provider
///     .resources
///     .insert(response.subject.to_string(), response);
///
/// let request = WebFingerRequest::builder("acct:alice@example.com")?
///     .host("example.com")
///     .rel("self")
///     .build();
///
/// let response = provider.resolve(&request).await?.unwrap();
/// assert_eq!(response.links.len(), 1);
/// # Ok(())
/// # }
/// ```
pub trait WebFingerProvider {
    /// Resolves a WebFinger request.
    ///
    /// Implementations may perform asynchronous I/O. Use the `request.resource` value for exact
    /// resource lookup and `request.rels` for relation filtering. Return `Ok(None)` for a valid
    /// request whose resource is not configured or otherwise unknown.
    fn resolve<'a>(
        &'a self,
        request: &'a WebFingerRequest,
    ) -> impl Future<Output = Result<Option<WebFingerResponse>, ProviderError>> + 'a;
}

/// A provider backed by a static parsed configuration.
///
/// This provider is useful for local servers, tests, examples, and deployments where configuration
/// is loaded before request handling begins. It applies the same exact resource matching and
/// relation filtering described by [`WebFingerProvider`].
#[derive(Debug, Clone)]
pub struct StaticConfigProvider {
    config: Config,
}

impl StaticConfigProvider {
    /// Parses a static TOML configuration.
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            config: Config::from_toml(input)?,
        })
    }

    /// Creates a provider from an already parsed configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Resolves a request against this provider without an async boundary.
    ///
    /// Use this when the caller is already inside synchronous code and does not need the
    /// [`WebFingerProvider`] abstraction.
    pub fn resolve_config(&self, request: &WebFingerRequest) -> Option<WebFingerResponse> {
        self.config.resolve(request)
    }
}

impl WebFingerProvider for StaticConfigProvider {
    /// Resolves a request against the parsed static configuration.
    async fn resolve<'a>(
        &'a self,
        request: &'a WebFingerRequest,
    ) -> Result<Option<WebFingerResponse>, ProviderError> {
        Ok(self.config.resolve(request))
    }
}

/// Errors raised while loading or resolving provider data.
///
/// Provider errors describe failures at the data boundary. Runtime adapters log detailed provider
/// errors, then decide how much information is safe to expose in HTTP responses.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ProviderError {
    /// A runtime binding was missing.
    #[error("missing runtime binding `{binding}`")]
    Binding {
        /// Binding name.
        binding: String,
        /// Safe runtime error message.
        message: String,
    },

    /// The configured key does not exist.
    #[error("missing configuration key `{key}`")]
    MissingConfig {
        /// Configuration key.
        key: String,
    },

    /// Reading the configured key failed.
    #[error("failed to read configuration key `{key}`: {message}")]
    ReadConfig {
        /// Configuration key.
        key: String,
        /// Safe runtime error message.
        message: String,
    },

    /// The configuration was invalid.
    #[error(transparent)]
    Config(#[from] ConfigError),
}
