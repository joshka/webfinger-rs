use std::io;

use clap::{Args, Parser};
use clap_cargo::style::CLAP_STYLING;
use clap_verbosity::{InfoLevel, Verbosity};
use color_eyre::eyre::{bail, Context};
use color_eyre::Result;
use colored_json::ToColoredJson;
use http::Uri;
use tracing::{debug, warn};
use tracing_log::AsTrace;
use webfinger_rs::{Rel, WebFingerRequest};

/// A simple CLI for fetching webfinger resources
#[derive(Debug, Parser)]
#[clap(styles = CLAP_STYLING)]
struct Cli {
    #[command(flatten)]
    fetch_command: FetchCommand,
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

#[derive(Debug, Args)]
#[command(next_line_help = false)]
struct FetchCommand {
    /// The resource to fetch
    ///
    /// E.g. `acct:user@example.com"
    resource: String,

    /// The host to fetch the webfinger resource from
    ///
    /// This defaults to the host part of the resource
    host: Option<String>,

    /// The link relation types to fetch
    ///
    /// E.g. `http://webfinger.net/rel/profile-page`
    ///
    /// This can be specified multiple times
    #[arg(short, long)]
    rel: Vec<String>,

    /// Ignore TLS certificate verification errors
    #[arg(long)]
    insecure: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();
    let log_level = args.verbosity.log_level_filter().as_trace();
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(io::stderr)
        .init();
    args.fetch_command.execute().await?;
    Ok(())
}

impl FetchCommand {
    async fn execute(&self) -> Result<()> {
        let request = WebFingerRequest {
            host: self.host()?,
            resource: self.resource()?,
            rels: self.link_relations(),
        };
        debug!("fetching webfinger resource: {:?}", request);
        if self.insecure {
            warn!("ignoring TLS certificate verification errors");
        }
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(self.insecure)
            .build()?;
        let response = request.execute_reqwest_with_client(&client).await?;
        let json = response.to_string().to_colored_json_auto()?;
        println!("{json}");
        Ok(())
    }

    fn host(&self) -> Result<String> {
        // TODO use correct normalization of host names
        if let Some(host) = self.host.as_deref() {
            Ok(host.to_string())
        } else if let Some((_, host)) = self.resource.split_once('@') {
            debug!("extracted host from resource: {}", host);
            Ok(host.to_string())
        } else {
            bail!("no host provided")
        }
    }

    fn resource(&self) -> Result<Uri> {
        self.resource.parse().wrap_err("invalid resource")
    }

    fn link_relations(&self) -> Vec<Rel> {
        self.rel.iter().map(|s| Rel::from(s.as_str())).collect()
    }
}
