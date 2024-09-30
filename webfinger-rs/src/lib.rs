pub use crate::{error::Error, types::*};

#[cfg(feature = "axum")]
mod axum;
mod error;
mod http;
#[cfg(feature = "reqwest")]
mod reqwest;
mod types;

pub const WELL_KNOWN_PATH: &str = "/.well-known/webfinger";
