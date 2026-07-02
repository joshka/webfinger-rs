use webfinger_rs::{WebFingerRequest, WebFingerResponse};

const AVATAR_REL: &str = "http://webfinger.net/rel/avatar";
const HOST: &str = "localhost:3000";
const PROFILE_PAGE_REL: &str = "http://webfinger.net/rel/profile-page";
const SUBJECT: &str = "acct:carol@localhost";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let request = WebFingerRequest::builder(SUBJECT)?
        .host(HOST)
        .rel(PROFILE_PAGE_REL)
        .rel(AVATAR_REL)
        .build();

    let reqwest_request = request.try_into_reqwest()?;
    eprintln!("GET {}", reqwest_request.url());

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .https_only(true)
        .build()?;
    let response = client.execute(reqwest_request).await?;
    let response = WebFingerResponse::try_from_reqwest(response).await?;

    println!("Subject: {}", response.subject);
    for rel in [PROFILE_PAGE_REL, AVATAR_REL] {
        if let Some(href) = response
            .links
            .iter()
            .find(|link| link.rel.as_ref() == rel)
            .and_then(|link| link.href.as_ref().map(|href| href.as_ref()))
        {
            println!("{rel}: {href}");
        }
    }
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
