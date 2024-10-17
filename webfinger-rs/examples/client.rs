use webfinger_rs::WebFingerRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = WebFingerRequest::builder("acct:carol@example.com")?
        .host("example.com")
        .rel("http://webfinger.net/rel/profile-page")
        .build();
    let response = request.execute().await?;
    dbg!(response);
    Ok(())
}
