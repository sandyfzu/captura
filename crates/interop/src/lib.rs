//! NAPI-rs bindings for xshot.
//!
//! This is the **only** layer that imports `napi` and `napi_derive`. It exposes
//! public functions to Node.js via `#[napi]` macros and handles conversion
//! between domain types and NAPI-compatible types.
//!
//! # Design Principles
//!
//! - Converts domain types into NAPI-compatible types for serialization.
//! - Converts Rust errors into JavaScript `Error` objects with structured codes.
//! - All exposed functions are `async` and return `Promise` to JavaScript.
//! - No `#[cfg]` attributes in this layer — all platform branching is resolved
//!   in the core or utility layers.
//! - Panics must never cross the FFI boundary.

mod error;
mod types;

use napi_derive::napi;

use types::{JsBase64CaptureResult, JsCaptureResult, JsMonitor};
use xshot_domain::ImageFormat;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns metadata for all connected monitors.
///
/// ```ts
/// const monitors: Monitor[] = await getMonitors()
/// ```
#[napi]
pub async fn get_monitors() -> napi::Result<Vec<JsMonitor>> {
    let monitors = xshot_core::get_monitors().await.map_err(error::to_napi)?;
    Ok(monitors.into_iter().map(JsMonitor::from).collect())
}

/// Returns metadata for the monitor with the given `id`.
///
/// Throws a `MONITOR_NOT_FOUND` error if no monitor matches.
///
/// ```ts
/// const monitor: Monitor = await getMonitorById(1)
/// ```
#[napi]
pub async fn get_monitor_by_id(id: u32) -> napi::Result<JsMonitor> {
    let info = xshot_core::get_monitor_by_id(id)
        .await
        .map_err(error::to_napi)?;
    Ok(JsMonitor::from(info))
}

/// Captures an encoded screenshot of the monitor with the given `id`.
///
/// Returns a `CaptureResult` containing monitor metadata and a `Screenshot`
/// with the image dimensions and encoded `Buffer`.
///
/// The optional `format` parameter selects the encoding. When omitted it
/// defaults to PNG (lossless, pixel-perfect). All formats use default
/// encoder settings — if you need fine-grained control over encoding
/// parameters, capture as PNG and re-encode with your preferred image
/// processing library.
///
/// ```ts
/// // Default (PNG)
/// const result: CaptureResult = await captureMonitor(1)
///
/// // Explicit format
/// const jpg: CaptureResult = await captureMonitor(1, 'Jpeg')
/// ```
#[napi]
pub async fn capture_monitor(
    id: u32,
    #[napi(ts_arg_type = "ImageFormat | (string & {})")] format: Option<String>,
) -> napi::Result<JsCaptureResult> {
    let fmt = parse_format(format)?;
    let result = xshot_core::capture_monitor(id, fmt)
        .await
        .map_err(error::to_napi)?;
    Ok(JsCaptureResult::from(result))
}

/// Captures encoded screenshots of every connected monitor.
///
/// The optional `format` parameter selects the encoding applied to all
/// captures. When omitted it defaults to PNG.
///
/// ```ts
/// const results: CaptureResult[] = await captureAllMonitors()
/// const avifResults: CaptureResult[] = await captureAllMonitors('Avif')
/// ```
#[napi]
pub async fn capture_all_monitors(
    #[napi(ts_arg_type = "ImageFormat | (string & {})")] format: Option<String>,
) -> napi::Result<Vec<JsCaptureResult>> {
    let fmt = parse_format(format)?;
    let results = xshot_core::capture_all_monitors(fmt)
        .await
        .map_err(error::to_napi)?;
    Ok(results.into_iter().map(JsCaptureResult::from).collect())
}

/// Captures a screenshot and returns it as a Base64-encoded string.
///
/// Identical to `captureMonitor()` except `screenshot.data` is a
/// [RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// Base64 string instead of a `Buffer`.
///
/// ```ts
/// const result: Base64CaptureResult = await captureMonitorBase64(1)
/// const dataUri = `data:image/png;base64,${result.screenshot.data}`
/// ```
#[napi]
pub async fn capture_monitor_base64(
    id: u32,
    #[napi(ts_arg_type = "ImageFormat | (string & {})")] format: Option<String>,
) -> napi::Result<JsBase64CaptureResult> {
    let fmt = parse_format(format)?;
    let result = xshot_core::capture_monitor_base64(id, fmt)
        .await
        .map_err(error::to_napi)?;
    Ok(JsBase64CaptureResult::from(result))
}

/// Captures Base64-encoded screenshots of every connected monitor.
///
/// Identical to `captureAllMonitors()` except each `screenshot.data` is a
/// [RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// Base64 string instead of a `Buffer`.
///
/// ```ts
/// const results: Base64CaptureResult[] = await captureAllMonitorsBase64()
/// ```
#[napi]
pub async fn capture_all_monitors_base64(
    #[napi(ts_arg_type = "ImageFormat | (string & {})")] format: Option<String>,
) -> napi::Result<Vec<JsBase64CaptureResult>> {
    let fmt = parse_format(format)?;
    let results = xshot_core::capture_all_monitors_base64(fmt)
        .await
        .map_err(error::to_napi)?;
    Ok(results
        .into_iter()
        .map(JsBase64CaptureResult::from)
        .collect())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parses an optional format string into an [`ImageFormat`], defaulting to
/// PNG when `None` is passed. Parsing is **case-insensitive** — `"png"`,
/// `"Png"`, `"PNG"` all resolve to [`ImageFormat::Png`], and `"jpg"` is
/// accepted as an alias for [`ImageFormat::Jpeg`].
///
/// Returns a [`napi::Error`] with an `INVALID_ARGUMENT` code if the string
/// does not match any supported format.
fn parse_format(format: Option<String>) -> napi::Result<ImageFormat> {
    match format {
        None => Ok(ImageFormat::Png),
        Some(s) => s.parse::<ImageFormat>().map_err(error::to_napi),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_format defaults to PNG -----------------------------------------

    #[test]
    fn parse_format_none_defaults_to_png() {
        let fmt = parse_format(None).expect("None should default to Png");
        assert_eq!(fmt, ImageFormat::Png);
    }

    // -- parse_format accepts canonical names ----------------------------------

    #[test]
    fn parse_format_canonical_png() {
        let fmt = parse_format(Some("Png".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Png);
    }

    #[test]
    fn parse_format_canonical_jpeg() {
        let fmt = parse_format(Some("Jpeg".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Jpeg);
    }

    #[test]
    fn parse_format_canonical_webp() {
        let fmt = parse_format(Some("WebP".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::WebP);
    }

    #[test]
    fn parse_format_canonical_avif() {
        let fmt = parse_format(Some("Avif".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Avif);
    }

    // -- parse_format is case-insensitive -------------------------------------

    #[test]
    fn parse_format_case_insensitive_uppercase() {
        let fmt = parse_format(Some("PNG".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Png);
    }

    #[test]
    fn parse_format_case_insensitive_lowercase() {
        let fmt = parse_format(Some("jpeg".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Jpeg);
    }

    #[test]
    fn parse_format_case_insensitive_mixed() {
        let fmt = parse_format(Some("wEbP".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::WebP);
    }

    // -- parse_format recognises "jpg" alias ----------------------------------

    #[test]
    fn parse_format_jpg_alias() {
        let fmt = parse_format(Some("jpg".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Jpeg);
    }

    #[test]
    fn parse_format_jpg_alias_uppercase() {
        let fmt = parse_format(Some("JPG".to_string())).unwrap();
        assert_eq!(fmt, ImageFormat::Jpeg);
    }

    // -- parse_format rejects unknown formats ---------------------------------

    #[test]
    fn parse_format_invalid_returns_error() {
        let err = parse_format(Some("bmp".to_string())).unwrap_err();
        assert!(
            err.reason.contains("[INVALID_ARGUMENT]"),
            "expected INVALID_ARGUMENT code, got: {}",
            err.reason
        );
    }

    #[test]
    fn parse_format_empty_string_returns_error() {
        let err = parse_format(Some(String::new())).unwrap_err();
        assert!(
            err.reason.contains("[INVALID_ARGUMENT]"),
            "expected INVALID_ARGUMENT code, got: {}",
            err.reason
        );
    }
}
