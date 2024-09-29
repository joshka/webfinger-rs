use clap::Parser;
use color_eyre::{
    eyre::{Context, OptionExt},
    Result,
};
use http::Uri;
use webfinger_rs::{LinkRelationType, Request};

#[derive(Debug, clap::Parser)]
struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, clap::Parser)]
enum Subcommand {
    #[clap(about = "Fetch a webfinger resource")]
    Fetch(FetchCommand),
}

#[derive(Debug, clap::Parser)]
struct FetchCommand {
    /// The resource to fetch
    resource: String,
    /// The host to fetch the webfinger resource from
    host: Option<String>,
    /// The link relation types to fetch
    #[arg(short, long)]
    rel: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    let args = Cli::parse();
    match args.subcommand {
        Subcommand::Fetch(command) => command.execute().await?,
    }
    Ok(())
}

impl FetchCommand {
    async fn execute(&self) -> Result<()> {
        let host = self.host()?;
        let resource = self.resource()?;
        let link_relation_types = self.link_relations();
        let query = Request {
            host,
            resource,
            link_relation_types,
        };
        let response = query.fetch().await?;
        println!("{:#?}", response);
        Ok(())
    }

    fn resource(&self) -> Result<Uri> {
        self.resource.parse().wrap_err("invalid resource")
    }

    fn host(&self) -> Result<String> {
        // TODO use correct normalization of host names
        let host = self
            .host
            .as_deref()
            .or_else(|| self.resource.split_once('@').map(|(_, host)| host))
            .ok_or_eyre("no host provided")?;
        host.parse()
            .wrap_err_with(|| format!("invalid host {:?}", host))
    }

    fn link_relations(&self) -> Vec<LinkRelationType> {
        self.rel
            .iter()
            .map(|s| LinkRelationType::from(s.as_str()))
            .collect()
    }
}
