#![cfg(all(feature = "axum", feature = "reqwest"))]

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Once;

use axum::Router;
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use http::StatusCode;
use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};

const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
const PROFILE_URL: &str = "https://localhost/users/carol";
const SUBJECT: &str = "acct:carol@localhost";

static DEFAULT_CRYPTO_PROVIDER: Once = Once::new();

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct TestServer {
    addr: SocketAddr,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn install_default_crypto_provider() {
    DEFAULT_CRYPTO_PROVIDER.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

async fn https_webfinger_server() -> TestResult<TestServer> {
    install_default_crypto_provider();

    let self_signed_cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert = self_signed_cert.cert.der().to_vec();
    let key = self_signed_cert.signing_key.serialize_der();
    let config = RustlsConfig::from_der(vec![cert.clone()], key).await?;
    let client = reqwest::Client::builder()
        .https_only(true)
        // The integration boundary here is the HTTP conversation over TLS, not platform PKI.
        // Windows rejects this rcgen self-signed certificate as an unknown issuer even when it is
        // added as a reqwest root, so keep certificate validation out of this portable test.
        .danger_accept_invalid_certs(true)
        .build()?;

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = TcpListener::bind(addr)?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let app = Router::new().route(WELL_KNOWN_PATH, get(webfinger));
    let server = axum_server::from_tcp_rustls(listener, config)?.serve(app.into_make_service());
    let task = tokio::spawn(async move {
        let _ = server.await;
    });

    Ok(TestServer { addr, client, task })
}

async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
    if request.resource.as_ref() != SUBJECT {
        return Err((StatusCode::NOT_FOUND, "not found").into());
    }

    let profile_rel = Rel::new(PROFILE_PAGE_REL);
    let links = if request.rels.is_empty() || request.rels.contains(&profile_rel) {
        vec![Link::builder(profile_rel).href(PROFILE_URL).build()]
    } else {
        Vec::new()
    };

    Ok(WebFingerResponse::builder(SUBJECT).links(links).build())
}

/// Exercises a complete HTTPS WebFinger conversation between this crate's Reqwest client path and
/// Axum server adapter.
///
/// The narrower response-shim unit tests cover status and JSON conversion without I/O. This test
/// crosses a localhost socket to catch integration breakage in URI construction, TLS transport,
/// Axum extraction, response serialization, and Reqwest decoding.
#[tokio::test]
async fn execute_reqwest_with_client_fetches_from_axum_server() -> TestResult {
    let server = https_webfinger_server().await?;
    let request = WebFingerRequest::builder(SUBJECT)?
        .host(format!("localhost:{}", server.addr.port()))
        .rel(PROFILE_PAGE_REL)
        .build();

    let response = request.execute_reqwest_with_client(&server.client).await?;

    assert_eq!(
        response,
        WebFingerResponse::builder(SUBJECT)
            .link(Link::builder(PROFILE_PAGE_REL).href(PROFILE_URL))
            .build()
    );
    Ok(())
}
