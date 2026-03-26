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

use types::{JsCaptureResult, JsImageFormat, JsMonitor};
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
    format: Option<JsImageFormat>,
) -> napi::Result<JsCaptureResult> {
    let fmt = format.map(ImageFormat::from).unwrap_or(ImageFormat::Png);
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
    format: Option<JsImageFormat>,
) -> napi::Result<Vec<JsCaptureResult>> {
    let fmt = format.map(ImageFormat::from).unwrap_or(ImageFormat::Png);
    let results = xshot_core::capture_all_monitors(fmt)
        .await
        .map_err(error::to_napi)?;
    Ok(results.into_iter().map(JsCaptureResult::from).collect())
}
