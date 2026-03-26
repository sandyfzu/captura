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

use types::{JsMonitor, JsScreenshot};

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

/// Captures a PNG-encoded screenshot of the monitor with the given `id`.
///
/// Returns a `Screenshot` containing monitor metadata and a `Buffer` with
/// the PNG data.
///
/// ```ts
/// const screenshot: Screenshot = await captureMonitor(1)
/// // screenshot.data is a Buffer containing PNG bytes
/// ```
#[napi]
pub async fn capture_monitor(id: u32) -> napi::Result<JsScreenshot> {
    let screenshot = xshot_core::capture_monitor(id)
        .await
        .map_err(error::to_napi)?;
    Ok(JsScreenshot::from(screenshot))
}

/// Captures PNG-encoded screenshots of every connected monitor.
///
/// ```ts
/// const screenshots: Screenshot[] = await captureAllMonitors()
/// ```
#[napi]
pub async fn capture_all_monitors() -> napi::Result<Vec<JsScreenshot>> {
    let screenshots = xshot_core::capture_all_monitors()
        .await
        .map_err(error::to_napi)?;
    Ok(screenshots.into_iter().map(JsScreenshot::from).collect())
}
