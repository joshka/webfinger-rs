//! WebFinger request and JRD response types.
//!
//! This module contains the Rust model for the protocol data described by RFC 7033:
//!
//! - [`Request`] models a WebFinger query from [RFC 7033 section 4.1].
//! - [`Response`] models the JSON Resource Descriptor (JRD) response from
//!   [RFC 7033 section 4.4].
//! - [`JrdUri`] is used where the JRD grammar calls for URI strings, including `subject`,
//!   `aliases`, `href`, and property identifiers.
//! - [`Rel`] is used where RFC 7033 requires a single link relation type rather than arbitrary
//!   text.
//! - [`Link`] and [`LinkBuilder`] model link objects from [RFC 7033 section 4.4.4].
//!
//! The public crate root re-exports these types under the common `WebFingerRequest` and
//! `WebFingerResponse` names, so most users can import from `webfinger_rs` directly.
//!
//! [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
//! [RFC 7033 section 4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4
//! [RFC 7033 section 4.4.4]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.4.4

pub use jrd_uri::JrdUri;
pub use link::{Link, LinkBuilder, Title};
pub use rel::Rel;
pub use request::{Builder as RequestBuilder, Request};
pub use resource::{Resource, ResourceError};
pub use response::{Builder as ResponseBuilder, Response};

mod jrd_uri;
mod link;
mod rel;
mod request;
mod resource;
mod response;
