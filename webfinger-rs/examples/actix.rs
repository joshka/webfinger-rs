use std::net::{Ipv4Addr, SocketAddr};

use actix_web::{get, App, HttpServer};
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use rustls::ServerConfig;
use tracing::{info, level_filters::LevelFilter};
use webfinger_rs::{Link, Rel, WebFingerRequest, WebFingerResponse, WELL_KNOWN_PATH};

const SUBJECT: &str = "acct:carol@localhost";

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let addrs = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000);
    let config = tls_config()?;
    let server =
        HttpServer::new(|| App::new().service(webfinger)).bind_rustls_0_23(addrs, config)?;
    info!("Listening at https://{addrs}{WELL_KNOWN_PATH}?resource={SUBJECT}");
    server.run().await?;
    Ok(())
}

fn tls_config() -> Result<ServerConfig> {
    let self_signed_cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .wrap_err("failed to generate self signed certificat for localhost")?;
    let cert_chain = self_signed_cert.cert.into();
    let key_der = self_signed_cert
        .key_pair
        .serialize_der()
        .try_into()
        .map_err(|s: &str| eyre!(s))?;
    // self_signed_cert.cert.
    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_chain], key_der)
        .wrap_err("failed to create tls config")
}

#[get("/.well-known/webfinger")]
async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
    info!("fetching webfinger resource: {:?}", request);
    let subject = request.resource.to_string();
    if subject != SUBJECT {
        let message = format!("{subject} does not exist");
        return Err(actix_web::error::ErrorNotFound(message))?;
    }
    let rel = Rel::new("http://webfinger.net/rel/profile-page");
    let response = if request.rels.is_empty() || request.rels.contains(&rel) {
        let link = Link::builder(rel).href(format!("https://example.com/profile/{subject}"));
        WebFingerResponse::builder(subject).link(link).build()
    } else {
        WebFingerResponse::builder(subject).build()
    };
    Ok(response)
}
