//! Header state shown outside the result fragment.
//!
//! htmx swaps the `#state` element out-of-band after each lookup. This module keeps the CSS class
//! and display text together so success, warning, and error states cannot drift between render
//! paths.

/// Header status shown outside the swapped result body.
///
/// htmx updates the `#state` element out-of-band, so this small view model keeps the CSS class and
/// text together at every render site.
pub struct StateView<'a> {
    /// CSS status class applied to the header state text.
    pub class_name: &'a str,

    /// User-visible state label.
    pub message: &'a str,
}

impl<'a> StateView<'a> {
    /// Creates a successful state label.
    ///
    /// Use this only for completed Worker lookups. Target HTTP failures still count as completed
    /// lookups because the viewer's job is to expose that target status in the result panel.
    pub fn good(message: &'a str) -> Self {
        Self {
            class_name: "good",
            message,
        }
    }

    /// Creates a warning state label for completed lookups with caveats.
    ///
    /// The current caveat is body truncation. Keeping this as a separate state makes it visible
    /// without changing the target status display, which should continue to reflect the server.
    pub fn warn(message: &'a str) -> Self {
        Self {
            class_name: "warn",
            message,
        }
    }

    /// Creates an error state label for viewer or Worker failures.
    ///
    /// This is not used for target WebFinger `404` or `500` responses; those are successful
    /// debugging lookups and should render through `lookup_result`.
    pub fn bad(message: &'a str) -> Self {
        Self {
            class_name: "bad",
            message,
        }
    }
}
