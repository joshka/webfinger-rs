use std::io;

use clap::{Args, Parser};
use clap_cargo::style::CLAP_STYLING;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use colored_json::ToColoredJson;
use tracing::{debug, warn};
use webfinger_rs::{Rel, Resource, WebFingerRequest};

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
    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .with_writer(io::stderr)
        .init();
    args.fetch_command.execute().await?;
    Ok(())
}

impl FetchCommand {
    async fn execute(&self) -> Result<()> {
        let resource = self.resource()?;
        let request = WebFingerRequest {
            host: self.host(&resource)?,
            resource,
            rels: self.link_relations()?,
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

    fn host(&self, resource: &Resource) -> Result<String> {
        if let Some(host) = self.host.as_deref() {
            Ok(host.to_string())
        } else {
            if let Some(host) = resource.host() {
                debug!("extracted host from resource URI: {}", host);
                Ok(host.to_string())
            } else if let Some((_, host)) = self.resource.split_once('@') {
                // TODO normalize account-address hosts before constructing the request URI.
                debug!("extracted host from acct resource: {}", host);
                Ok(host.to_string())
            } else {
                bail!("no host provided")
            }
        }
    }

    fn resource(&self) -> Result<Resource> {
        self.resource.parse().wrap_err("invalid resource")
    }

    fn link_relations(&self) -> Result<Vec<Rel>> {
        self.rel
            .iter()
            .map(Rel::try_new)
            .collect::<std::result::Result<Vec<_>, _>>()
            .wrap_err("invalid relation type")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a fetch command with only the resource set.
    fn command(resource: &str) -> FetchCommand {
        FetchCommand {
            resource: resource.to_string(),
            host: None,
            rel: Vec::new(),
            insecure: false,
        }
    }

    /// Uses the URI authority when the resource has one.
    ///
    /// WebFinger resources are not always `acct:` URIs. Parsing the URI before falling back to
    /// `acct:` splitting prevents `@` inside an HTTPS path from being mistaken for the request host.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[test]
    fn host_uses_resource_uri_authority() {
        let command = command("https://example.org/users/@carol");
        let resource = command.resource().unwrap();

        let host = command.host(&resource).unwrap();

        assert_eq!(host, "example.org");
    }

    /// Falls back to the account authority for `acct:` resources.
    ///
    /// `acct:` URIs do not expose a URI host through the `http::Uri` API, so the CLI keeps the
    /// WebFinger account-address fallback for the common `acct:user@example.org` case.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7565.html#section-3>.
    #[test]
    fn host_falls_back_to_acct_authority() {
        let command = command("acct:carol@example.org");
        let resource = command.resource().unwrap();

        let host = command.host(&resource).unwrap();

        assert_eq!(host, "example.org");
    }

    /// Rejects non-hierarchical HTTP resource text before host inference.
    #[test]
    fn resource_rejects_http_uri_without_authority() {
        let command = command("http:foo");

        let error = command
            .resource()
            .expect_err("HTTP resource without authority");

        assert!(error.to_string().contains("invalid resource"));
    }
}
