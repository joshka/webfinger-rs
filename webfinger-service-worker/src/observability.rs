//! Worker logging setup.
//!
//! Cloudflare Worker Logs capture messages written to the JavaScript console. The service uses
//! `tracing` for request and provider breadcrumbs, so wasm builds install a console-backed tracing
//! subscriber at the Worker boundary.

/// Installs logging support for the current runtime.
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
