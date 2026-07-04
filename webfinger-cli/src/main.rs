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

#[derive(Debug, Args, PartialEq, Eq)]
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
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
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

    /// Honors an explicit CLI host over any host that could be inferred from the resource.
    ///
    /// WebFinger deployments can serve an account domain from a different endpoint host, so the
    /// caller-provided host must remain authoritative when present.
    #[test]
    fn host_uses_explicit_host() {
        let mut command = command("acct:carol@example.org");
        command.host = Some("webfinger.example.net".to_string());
        let resource = command.resource().unwrap();

        let host = command.host(&resource).unwrap();

        assert_eq!(host, "webfinger.example.net");
    }

    /// Rejects host inference when neither an HTTP(S) authority nor an `acct:` authority is
    /// available.
    ///
    /// RFC 7033 section 4.1 requires a concrete WebFinger endpoint host for the outgoing query.
    /// Without an explicit CLI host, only hierarchical resource URIs and `acct:` account addresses
    /// provide enough information to infer that endpoint.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    /// See <https://www.rfc-editor.org/rfc/rfc7565.html#section-3>.
    #[test]
    fn host_rejects_resource_without_inferable_host() {
        let command = command("mailto:carol");
        let resource = command.resource().unwrap();

        let error = command.host(&resource).expect_err("missing host");

        assert_eq!(error.to_string(), "no host provided");
    }

    /// Parses all relation filters through the same validated `Rel` boundary used by the library.
    ///
    /// Repeated `--rel` options become repeated WebFinger `rel` query parameters, so their order is
    /// observable in the generated request URI.
    #[test]
    fn link_relations_parse_in_order() {
        let mut command = command("acct:carol@example.org");
        command.rel = vec![
            "http://webfinger.net/rel/profile-page".to_string(),
            "avatar".to_string(),
        ];

        let rels = command.link_relations().unwrap();

        assert_eq!(
            rels.iter().map(Rel::as_ref).collect::<Vec<_>>(),
            ["http://webfinger.net/rel/profile-page", "avatar"]
        );
    }

    /// Rejects invalid CLI relation filters before a network request can be built.
    ///
    /// This keeps CLI validation consistent with response and request relation validation instead
    /// of letting malformed relation strings reach the transport layer.
    #[test]
    fn link_relations_reject_invalid_rel() {
        let mut command = command("acct:carol@example.org");
        command.rel = vec!["profile page".to_string()];

        let error = command.link_relations().expect_err("invalid rel");

        assert!(error.to_string().contains("invalid relation type"));
    }

    /// Stops invalid resource text before the CLI constructs a WebFinger request.
    ///
    /// The command path should fail with local validation context, not with a later Reqwest URL or
    /// transport error that hides the malformed resource.
    #[tokio::test]
    async fn execute_rejects_invalid_resource_before_network() {
        let command = command("http:foo");

        let error = command.execute().await.expect_err("invalid resource");

        assert!(error.to_string().contains("invalid resource"));
    }

    /// Stops execution when the endpoint host cannot be explicit or inferred.
    ///
    /// This covers the full async command path for the same missing-host boundary tested directly
    /// by `host_rejects_resource_without_inferable_host`.
    #[tokio::test]
    async fn execute_rejects_missing_host_before_network() {
        let command = command("mailto:carol");

        let error = command.execute().await.expect_err("missing host");

        assert_eq!(error.to_string(), "no host provided");
    }

    /// Validates relation filters before initializing the HTTP client.
    ///
    /// A bad `--rel` value should be reported as command input failure even when the resource and
    /// host are otherwise usable.
    #[tokio::test]
    async fn execute_rejects_invalid_relation_before_network() {
        let mut command = command("acct:carol@example.org");
        command.host = Some("example.org".to_string());
        command.rel = vec!["profile page".to_string()];

        let error = command.execute().await.expect_err("invalid rel");

        assert!(error.to_string().contains("invalid relation type"));
    }

    /// Locks the positional CLI shape for resource, optional host, repeated rels, and TLS override.
    ///
    /// Clap derives this parser from struct field order and attributes, so a small parser refactor
    /// can unintentionally change the public command line.
    #[test]
    fn cli_parses_resource_host_rel_and_insecure_flag() {
        let cli = Cli::try_parse_from([
            "webfinger",
            "acct:carol@example.org",
            "example.org",
            "--rel",
            "avatar",
            "--insecure",
        ])
        .unwrap();

        assert_eq!(
            cli.fetch_command,
            FetchCommand {
                resource: "acct:carol@example.org".to_string(),
                host: Some("example.org".to_string()),
                rel: vec!["avatar".to_string()],
                insecure: true,
            }
        );
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
