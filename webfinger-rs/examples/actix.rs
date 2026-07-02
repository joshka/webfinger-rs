use std::net::{Ipv4Addr, SocketAddr};

use actix_web::{App, HttpServer, web};
use color_eyre::Result;
use color_eyre::eyre::{Context, eyre};
use rustls::ServerConfig;
use tracing::info;
use tracing_subscriber::EnvFilter;
use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};

const AVATAR_HREF: &str = "https://localhost:3000/media/carol.png";
const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
const HOST: &str = "localhost:3000";
const PROFILE_HREF: &str = "https://localhost:3000/users/carol";
const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
const ROLE_PROPERTY: &str = "https://example.com/ns/account-role";
const SUBJECT: &str = "acct:carol@localhost";

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let addrs = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000);
    let config = tls_config()?;
    let server = HttpServer::new(|| App::new().route(WELL_KNOWN_PATH, web::get().to(webfinger)))
        .bind_rustls_0_23(addrs, config)?;
    let unfiltered_request = WebFingerRequest::builder(SUBJECT)?.host(HOST).build();
    let profile_request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(PROFILE_PAGE_REL)
        .build();
    let avatar_request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(AVATAR_REL)
        .build();

    info!("Listening at https://{addrs}{WELL_KNOWN_PATH}");
    info!(
        "Unfiltered query: {}",
        http::Uri::try_from(&unfiltered_request)?
    );
    info!(
        "Profile-page query: {}",
        http::Uri::try_from(&profile_request)?
    );
    info!("Avatar query: {}", http::Uri::try_from(&avatar_request)?);
    server.run().await?;
    Ok(())
}

fn tls_config() -> Result<ServerConfig> {
    let self_signed_cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .wrap_err("failed to generate self signed certificate for localhost")?;
    let cert_chain = self_signed_cert.cert.into();
    let key_der = self_signed_cert
        .signing_key
        .serialize_der()
        .try_into()
        .map_err(|s: &str| eyre!(s))?;
    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_chain], key_der)
        .wrap_err("failed to create tls config")
}

async fn webfinger(request: WebFingerRequest) -> actix_web::Result<WebFingerResponse> {
    info!("fetching webfinger resource: {:?}", request);
    let subject = request.resource.to_string();
    if subject != SUBJECT {
        let message = format!("{subject} does not exist");
        return Err(actix_web::error::ErrorNotFound(message))?;
    }
    let mut links = Vec::new();

    let profile_rel = Rel::new(PROFILE_PAGE_REL);
    if request.rels.is_empty() || request.rels.contains(&profile_rel) {
        links.push(
            Link::builder(profile_rel)
                .href(PROFILE_HREF)
                .title("en", "Carol's profile")
                .build(),
        );
    }

    let avatar_rel = Rel::new(AVATAR_REL);
    if request.rels.is_empty() || request.rels.contains(&avatar_rel) {
        links.push(
            Link::builder(avatar_rel)
                .href(AVATAR_HREF)
                .r#type("image/png")
                .build(),
        );
    }

    let response = WebFingerResponse::builder(subject)
        .alias(PROFILE_HREF)
        .property(ROLE_PROPERTY, "maintainer")
        .links(links)
        .build();
    Ok(response)
}
