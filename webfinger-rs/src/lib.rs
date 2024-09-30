pub use crate::{
    error::Error,
    rel::Rel,
    request::Request,
    response::{Link, Response, Title},
};

#[cfg(feature = "axum")]
mod axum;
mod error;
mod http;
mod rel;
mod request;
#[cfg(feature = "reqwest")]
mod reqwest;
mod response;

pub const WELL_KNOWN_PATH: &str = "/.well-known/webfinger";
