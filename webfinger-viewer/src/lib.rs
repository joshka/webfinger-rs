//! Shared WebFinger viewer behavior.
//!
//! This crate owns the runtime-neutral viewer behavior, Askama templates, WebFinger lookup policy,
//! and result shaping. Native Axum deployment lives in `webfinger-viewer-axum`, and Cloudflare
//! Worker deployment lives in `webfinger-viewer-worker`.

pub mod app;
pub mod config;
pub mod lookup;
mod view;
