//! Worker logging setup.
//!
//! Cloudflare Worker Logs capture messages written to the JavaScript console. The Rust code uses
//! `tracing` for request and lookup breadcrumbs, so wasm builds install a console-backed tracing
//! subscriber at the fetch boundary. Native tests do not need a global subscriber; leaving them
//! unconfigured avoids fights with test harnesses or future crate-level tracing capture.

/// Installs logging support for the current runtime.
///
/// This is intentionally safe to call for every fetch event. Worker isolates can handle many
/// requests, but they can also be restarted at any time; a tiny idempotent initializer keeps the
/// entrypoint simple and makes validation obvious: emit a `tracing::info!` event and confirm it
/// appears in Wrangler tail or Cloudflare dashboard logs.
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    init_wasm_console_tracing();
}

#[cfg(target_arch = "wasm32")]
fn init_wasm_console_tracing() {
    use std::sync::Once;
    use tracing_subscriber::prelude::*;

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(tracing_web::MakeWebConsoleWriter::new());

        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}
