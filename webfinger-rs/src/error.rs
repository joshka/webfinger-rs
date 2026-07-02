use crate::ResourceError;

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

    /// A WebFinger resource is malformed.
    #[error(transparent)]
    InvalidResource(#[from] ResourceError),

    /// A WebFinger JRD field expected an absolute URI string.
    #[error("invalid JRD URI: {0}")]
    InvalidJrdUri(String),

    /// A WebFinger relation type was not a URI or registered relation type.
    #[error("invalid relation type: {0}")]
    InvalidRel(String),
}
