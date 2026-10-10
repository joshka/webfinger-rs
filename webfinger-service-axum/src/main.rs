use std::path::{Path, PathBuf};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::SocketAddr;

    use tracing::info;
    use webfinger_service::StaticConfigProvider;
    use webfinger_service_axum::axum_router;

    let _ = tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_env_filter(log_filter())
        .try_init();

    let args = Args::parse();
    let config_source = args.config_source();
    let config = config_source.load_config()?;
    let provider = StaticConfigProvider::from_toml(&config)?;

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| bind_error(addr, error))?;

    eprintln!(
        "serving webfinger service at http://{addr}/.well-known/webfinger using {}",
        config_source
    );
    info!(%addr, config = %config_source, "serving webfinger service");
    axum::serve(listener, axum_router(provider)).await?;
    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Configuration TOML file to serve.
    #[arg(long, env = "WEBFINGER_CONFIG_FILE", value_name = "PATH")]
    config: Option<PathBuf>,

    /// Serve the bundled example configuration instead of the default empty config.
    #[arg(long, conflicts_with = "config")]
    example_config: bool,

    /// Host address to bind.
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// TCP port to bind.
    #[arg(long, env = "PORT", default_value_t = 8788)]
    port: u16,
}

impl Args {
    fn config_source(&self) -> ConfigSource {
        if self.example_config {
            ConfigSource::Example
        } else if let Some(path) = &self.config {
            ConfigSource::File(path.clone())
        } else {
            ConfigSource::Defaults
        }
    }
}

enum ConfigSource {
    Defaults,
    Example,
    File(PathBuf),
}

impl ConfigSource {
    fn load_config(&self) -> Result<String, CliError> {
        match self {
            Self::Defaults => Ok("resources = []".to_string()),
            Self::Example => Ok(webfinger_service::EXAMPLE_CONFIG.to_string()),
            Self::File(path) => read_config(path),
        }
    }
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Defaults => formatter.write_str("built-in defaults"),
            Self::Example => formatter.write_str("embedded example"),
            Self::File(path) => write!(formatter, "{}", path.display()),
        }
    }
}

fn read_config(path: &Path) -> Result<String, CliError> {
    std::fs::read_to_string(path).map_err(|error| {
        CliError(format!(
            "could not read WebFinger config from {}: {error}",
            path.display()
        ))
    })
}

fn bind_error(addr: std::net::SocketAddr, error: std::io::Error) -> CliError {
    if error.kind() == std::io::ErrorKind::AddrInUse {
        CliError(format!(
            "could not start webfinger-service-axum because {addr} is already in use; \
             stop the process using that port or run with PORT=<free-port>"
        ))
    } else {
        CliError(format!(
            "could not bind webfinger-service-axum to {addr}: {error}"
        ))
    }
}

struct CliError(String);

impl std::fmt::Debug for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

fn log_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addr_in_use_error_names_address_and_fix() {
        let addr = "127.0.0.1:8787".parse().unwrap();
        let error = bind_error(
            addr,
            std::io::Error::new(std::io::ErrorKind::AddrInUse, "in use"),
        );

        let message = error.to_string();
        assert!(message.contains("127.0.0.1:8787"));
        assert!(message.contains("PORT=<free-port>"));
    }

    #[test]
    fn explicit_config_conflicts_with_example() {
        let error =
            Args::try_parse_from(["webfinger", "--config", "config.toml", "--example-config"])
                .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn no_config_uses_built_in_defaults() {
        let args = Args::try_parse_from(["webfinger"]).unwrap();
        let source = args.config_source();
        assert_eq!(source.to_string(), "built-in defaults");
        let config = source.load_config().unwrap();
        webfinger_service::StaticConfigProvider::from_toml(&config).unwrap();
        assert_eq!(config, "resources = []");
    }

    #[test]
    fn example_config_flag_loads_bundled_example() {
        let args = Args {
            config: None,
            example_config: true,
            host: "127.0.0.1".to_string(),
            port: 8788,
        };

        let source = args.config_source();
        assert_eq!(source.to_string(), "embedded example");
        source.load_config().unwrap();
    }
}
