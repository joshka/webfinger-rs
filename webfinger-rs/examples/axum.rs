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

const HOST: &str = "localhost:3000";
const PROFILE_HREF: &str = "https://example.com/users/carol";
const PROFILE_REL: &str = "http://webfinger.net/rel/profile-page";
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
        .rel(PROFILE_REL)
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
    let rel = Rel::new(PROFILE_REL);
    let response = if request.rels.is_empty() || request.rels.contains(&rel) {
        let link = Link::builder(rel).href(PROFILE_HREF);
        WebFingerResponse::builder(subject).link(link).build()
    } else {
        WebFingerResponse::builder(subject).build()
    };
    Ok(response)
}
