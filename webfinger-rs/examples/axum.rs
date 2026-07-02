use std::net::{Ipv4Addr, SocketAddr};

use axum::Router;
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use color_eyre::Result;
use color_eyre::eyre::Context;
use http::StatusCode;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing::level_filters::LevelFilter;
use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};

const AVATAR_HREF: &str = "https://localhost:3000/media/carol.png";
const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
const HOST: &str = "localhost:3000";
const PROFILE_HREF: &str = "https://localhost:3000/users/carol";
const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
const ROLE_PROPERTY: &str = "https://example.com/ns/account-role";
const SUBJECT: &str = "acct:carol@localhost";

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000);
    let router = Router::new()
        .route(WELL_KNOWN_PATH, get(webfinger))
        .route_layer(TraceLayer::new_for_http())
        .into_make_service();
    let config = tls_config().await?;
    let unfiltered_request = WebFingerRequest::builder(SUBJECT)?.host(HOST).build();
    let profile_request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(PROFILE_PAGE_REL)
        .build();
    let avatar_request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(AVATAR_REL)
        .build();

    info!("Listening at https://{addr}{WELL_KNOWN_PATH}");
    info!(
        "Unfiltered query: {}",
        http::Uri::try_from(&unfiltered_request)?
    );
    info!(
        "Profile-page query: {}",
        http::Uri::try_from(&profile_request)?
    );
    info!("Avatar query: {}", http::Uri::try_from(&avatar_request)?);
    axum_server::bind_rustls(addr, config).serve(router).await?;

    Ok(())
}

/// Generate a self-signed certificate for localhost
async fn tls_config() -> Result<RustlsConfig> {
    let self_signed_cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .wrap_err("failed to generate self signed certificate for localhost")?;
    let cert = self_signed_cert.cert.der().to_vec();
    let key = self_signed_cert.signing_key.serialize_der();
    RustlsConfig::from_der(vec![cert], key)
        .await
        .wrap_err("failed to create tls config")
}

async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
    info!("fetching webfinger resource: {:?}", request);
    let subject = request.resource.to_string();
    if subject != SUBJECT {
        let message = format!("{subject} does not exist");
        return Err((StatusCode::NOT_FOUND, message).into());
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
