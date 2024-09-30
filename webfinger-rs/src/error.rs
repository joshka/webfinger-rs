#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] http::Error),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
