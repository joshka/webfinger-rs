use std::path::{Path, PathBuf};

use clap::Parser;

const DEFAULT_CONFIG_FILE: &str = "webfinger-service/webfinger.toml";
const EXAMPLE_CONFIG_FILE: &str = "webfinger-service/webfinger.example.toml";

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
    let config_path = args.config_path();
    let config = read_config(config_path)?;
    let provider = StaticConfigProvider::from_toml(&config)?;

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| bind_error(addr, error))?;

    eprintln!(
        "serving webfinger service at http://{addr}/.well-known/webfinger using {}",
        config_path.display()
    );
    info!(%addr, config = %config_path.display(), "serving webfinger service");
    axum::serve(listener, axum_router(provider)).await?;
    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Configuration TOML file to serve.
    #[arg(
        long,
        env = "WEBFINGER_CONFIG_FILE",
        value_name = "PATH",
        default_value = DEFAULT_CONFIG_FILE
    )]
    config: PathBuf,

    /// Serve the bundled example configuration instead of the default empty config.
    #[arg(long)]
    example_config: bool,

    /// Host address to bind.
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// TCP port to bind.
    #[arg(long, env = "PORT", default_value_t = 8788)]
    port: u16,
}

impl Args {
    fn config_path(&self) -> &Path {
        if self.example_config {
            Path::new(EXAMPLE_CONFIG_FILE)
        } else {
            &self.config
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
    fn empty_default_config_parses() {
        webfinger_service::StaticConfigProvider::from_toml(include_str!(
            "../../webfinger-service/webfinger.toml"
        ))
        .unwrap();
    }

    #[test]
    fn example_config_flag_selects_example_file() {
        let args = Args {
            config: DEFAULT_CONFIG_FILE.into(),
            example_config: true,
            host: "127.0.0.1".to_string(),
            port: 8788,
        };

        assert_eq!(args.config_path(), Path::new(EXAMPLE_CONFIG_FILE));
    }
}
