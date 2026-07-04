//! Native Axum entrypoint for the WebFinger viewer.

mod axum;

#[cfg(not(target_arch = "wasm32"))]
use std::{
    net::Ipv6Addr,
    path::{Path, PathBuf},
};

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
use tracing::info;
#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::EnvFilter;
#[cfg(not(target_arch = "wasm32"))]
use webfinger_viewer::config::{ServerConfig, ViewerConfig};

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_CONFIG_FILE: &str = "webfinger-viewer/viewer.toml";
#[cfg(not(target_arch = "wasm32"))]
const EXAMPLE_CONFIG_FILE: &str = "webfinger-viewer/viewer.example.toml";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_HOST: &str = "127.0.0.1";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_PORT: u16 = 8788;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let args = Args::parse();
    let config_path = args.config_path();
    let config = read_config(config_path)?;
    let server = server_config(&config, &args);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let app = crate::axum::router(client, config.lookup.clone());
    let addr = server_addr(&server);
    let listener = tokio::net::TcpListener::bind(addr.as_str())
        .await
        .map_err(|error| bind_error(&addr, error))?;

    eprintln!(
        "serving webfinger viewer at http://{addr}/webfinger using {}",
        config_path.display()
    );
    info!(%addr, config = %config_path.display(), "serving webfinger viewer");
    ::axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_env_filter(filter)
        .try_init();
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Configuration TOML file for local viewer defaults.
    #[arg(
        long,
        env = "WEBFINGER_VIEWER_CONFIG_FILE",
        value_name = "PATH",
        default_value = DEFAULT_CONFIG_FILE
    )]
    config: PathBuf,

    /// Use the bundled example viewer configuration.
    #[arg(long)]
    example_config: bool,

    /// Host address to bind.
    #[arg(long, env = "HOST")]
    host: Option<String>,

    /// TCP port to bind.
    #[arg(long, env = "PORT")]
    port: Option<u16>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Args {
    fn config_path(&self) -> &Path {
        if self.example_config {
            Path::new(EXAMPLE_CONFIG_FILE)
        } else {
            &self.config
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_config(path: &Path) -> Result<ViewerConfig, CliError> {
    let config = std::fs::read_to_string(path).map_err(|error| {
        CliError(format!(
            "could not read WebFinger viewer config from {}: {error}",
            path.display()
        ))
    })?;
    ViewerConfig::from_toml(&config).map_err(|error| {
        CliError(format!(
            "invalid WebFinger viewer config in {}: {error}",
            path.display()
        ))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn server_config(config: &ViewerConfig, args: &Args) -> ServerConfig {
    let mut server = config.server.clone();
    if let Some(host) = &args.host {
        server.host = Some(host.clone());
    }
    if let Some(port) = args.port {
        server.port = Some(port);
    }
    server
}

#[cfg(not(target_arch = "wasm32"))]
fn server_addr(server: &ServerConfig) -> String {
    let host = server.host.as_deref().unwrap_or(DEFAULT_HOST);
    let port = server.port.unwrap_or(DEFAULT_PORT);
    format!("{}:{port}", socket_host(host))
}

#[cfg(not(target_arch = "wasm32"))]
fn socket_host(host: &str) -> String {
    let unbracketed_host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if unbracketed_host.parse::<Ipv6Addr>().is_ok() {
        format!("[{unbracketed_host}]")
    } else {
        host.to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn bind_error(addr: &str, error: std::io::Error) -> CliError {
    if error.kind() == std::io::ErrorKind::AddrInUse {
        CliError(format!(
            "could not start webfinger-viewer-axum because {addr} is already in use; \
             stop the process using that port or run with PORT=<free-port>"
        ))
    } else {
        CliError(format!(
            "could not bind webfinger-viewer-axum to {addr}: {error}"
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CliError(String);

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Debug for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for CliError {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn example_config_flag_selects_example_file() {
        let args = Args {
            config: DEFAULT_CONFIG_FILE.into(),
            example_config: true,
            host: None,
            port: None,
        };

        assert_eq!(args.config_path(), Path::new(EXAMPLE_CONFIG_FILE));
    }

    #[test]
    fn default_config_matches_checked_in_local_defaults() {
        let config =
            ViewerConfig::from_toml(include_str!("../../webfinger-viewer/viewer.toml")).unwrap();

        assert_eq!(config.server.host.as_deref(), Some(DEFAULT_HOST));
        assert_eq!(config.server.port, Some(DEFAULT_PORT));
        assert_eq!(config.lookup.local_responder_port, 8787);
    }

    #[test]
    fn example_config_changes_bind_and_lookup_defaults() {
        let config =
            ViewerConfig::from_toml(include_str!("../../webfinger-viewer/viewer.example.toml"))
                .unwrap();

        assert_eq!(config.server.host.as_deref(), Some(DEFAULT_HOST));
        assert_eq!(config.server.port, Some(8791));
        assert_eq!(config.lookup.local_responder_port, 8790);
    }

    #[test]
    fn cli_host_and_port_override_config() {
        let config = ViewerConfig {
            server: ServerConfig {
                host: Some("127.0.0.1".to_string()),
                port: Some(8788),
            },
            lookup: Default::default(),
        };
        let args = Args {
            config: DEFAULT_CONFIG_FILE.into(),
            example_config: false,
            host: Some("0.0.0.0".to_string()),
            port: Some(8790),
        };

        let server = server_config(&config, &args);

        assert_eq!(server.host.as_deref(), Some("0.0.0.0"));
        assert_eq!(server.port, Some(8790));
    }

    #[test]
    fn addr_in_use_error_names_address_and_fix() {
        let error = bind_error(
            "127.0.0.1:8787",
            std::io::Error::new(std::io::ErrorKind::AddrInUse, "in use"),
        );

        let message = error.to_string();
        assert!(message.contains("127.0.0.1:8787"));
        assert!(message.contains("PORT=<free-port>"));
    }

    #[test]
    fn server_addr_brackets_ipv6_bind_host() {
        let server = ServerConfig {
            host: Some("::1".to_string()),
            port: Some(8790),
        };

        assert_eq!(server_addr(&server), "[::1]:8790");
    }

    #[test]
    fn server_addr_preserves_hostname_bind_host() {
        let server = ServerConfig {
            host: Some("localhost".to_string()),
            port: Some(8790),
        };

        assert_eq!(server_addr(&server), "localhost:8790");
    }

    #[test]
    fn missing_config_error_names_path() {
        let path = Path::new("webfinger-viewer/missing.toml");
        let error = read_config(path).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("could not read WebFinger viewer config"));
        assert!(message.contains("webfinger-viewer/missing.toml"));
    }

    #[test]
    fn invalid_config_error_names_path() {
        let temp_dir = std::env::temp_dir().join(format!(
            "webfinger-viewer-axum-config-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("invalid.toml");
        std::fs::write(&path, "[lookup]\nlocal_responder_port = \"not-a-port\"\n").unwrap();

        let error = read_config(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&temp_dir).unwrap();

        let message = error.to_string();
        assert!(message.contains("invalid WebFinger viewer config"));
        assert!(message.contains("invalid.toml"));
        assert!(message.contains("local_responder_port"));
    }
}
