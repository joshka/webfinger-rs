use std::net::Ipv4Addr;

use axum::{routing::get, Router};
use color_eyre::Result;
use http::StatusCode;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use webfinger_rs::{
    Link, Rel, Request as WebFingerRequest, Response as WebFingerResponse, WELL_KNOWN_PATH,
};

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

async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
    let subject = request.resource.to_string();
    if subject != "acct:carol@example.com" {
        return Err(not_found().await.into());
    }
    let rel = Rel::new("http://webfinger.net/rel/profile-page");
    if !request.rels.is_empty() && !request.rels.contains(&rel) {
        return Ok(WebFingerResponse::builder(subject).build());
    }
    let link = Link::builder(rel).href(format!("https://example.com/profile/{subject}"));
    let reponse = WebFingerResponse::builder(subject).link(link).build();
    Ok(reponse)
}

async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}
