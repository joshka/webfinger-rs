use webfinger_rs::{WebFingerRequest, WebFingerResponse};

const HOST: &str = "localhost:3000";
const PROFILE_REL: &str = "http://webfinger.net/rel/profile-page";
const SUBJECT: &str = "acct:carol@localhost";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(PROFILE_REL)
        .build();

    let reqwest_request = request.try_into_reqwest()?;
    eprintln!("GET {}", reqwest_request.url());

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .https_only(true)
        .build()?;
    let response = client.execute(reqwest_request).await?;
    let response = WebFingerResponse::try_from_reqwest(response).await?;

    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
