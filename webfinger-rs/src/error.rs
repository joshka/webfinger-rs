/// Error type for this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while performing an HTTP request.
    #[error(transparent)]
    Http(#[from] http::Error),

    /// An error occurred while sending an HTTP request using `reqwest`.
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// An error occurred while parsing JSON.
    // #[error("json error: {0}")]
    // Json(#[from] serde_json::Error),
    #[error("invalid uri: {0}")]
    InvalidUri(#[from] http::uri::InvalidUri),
}
