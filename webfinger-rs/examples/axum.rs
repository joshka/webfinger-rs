use std::net::Ipv4Addr;

use axum::{routing::get, Router};
use color_eyre::Result;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use webfinger_rs::{Link, Request as WebFingerRequest, Response as WebFingerResponse};

const WELL_KNOWN_PATH: &str = "/.well-known/webfinger";

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let listener = bind().await?;
    let router = router();
    axum::serve(listener, router).await?;

    Ok(())
}

async fn bind() -> Result<TcpListener> {
    let addr = (Ipv4Addr::LOCALHOST, 3000);
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    info!("Listening at http://{local_addr:?}{WELL_KNOWN_PATH}?resource=acct:carol@example.com");
    Ok(listener)
}

fn router() -> Router {
    Router::new()
        .route(WELL_KNOWN_PATH, get(webfinger))
        .fallback(not_found)
        .route_layer(TraceLayer::new_for_http())
}

async fn webfinger(request: WebFingerRequest) -> WebFingerResponse {
    let subject = request.resource.to_string();
    let mut link = Link::new("http://webfinger.net/rel/profile-page".into());
    link.href = Some(format!("https://example.com/{subject}"));
    let links = vec![link];
    WebFingerResponse {
        subject,
        links,
        aliases: None,
        properties: None,
    }
}

async fn not_found() -> &'static str {
    "Not Found"
}
