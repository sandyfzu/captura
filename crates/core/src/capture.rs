//! Monitor discovery and screen-capture orchestration.
//!
//! Every public function in this module runs on a blocking Tokio thread
//! (`spawn_blocking`) because `xcap` uses synchronous OS APIs that must
//! not execute on the async runtime's cooperative threads.
//!
//! # Platform normalisation
//!
//! `xcap` v0.9.3 reports monitor geometry in different coordinate systems
//! depending on the platform:
//!
//! | Platform | xcap x / y / width / height | Coordinate system |
//! |---|---|---|
//! | macOS | `CGDisplayBounds()` | **Logical points** |
//! | Linux | `XRandR raw ÷ scale_factor` | **Logical** |
//! | Windows | `DEVMODEW.dmPelsWidth` / `dmPosition` | **Physical pixels** |
//!
//! All captured screenshots (`capture_image()`) are in **physical pixels**
//! on every platform. This module normalises all reported dimensions to
//! physical pixels so that `width` / `height` always match the captured
//! image dimensions, and the API surface is consistent across operating
//! systems.
//!
//! ## Normalisation applied
//!
//! | Concern | Normalisation |
//! |---|---|
//! | `x`, `y`, `width`, `height` | Converted to **physical pixels** (macOS / Linux: `× scale_factor`; Windows: passthrough) |
//! | `rotation` | `f32` → `f64` (lossless widening) |
//! | `scale_factor` | `f32` → `f64` |
//! | `frequency` | `f32` → `f64` |
//! | `is_builtin` | Not available on all platforms — falls back to `false` |
//!
//! ## Sources
//!
//! - macOS: `CGDisplayBounds` returns logical points — [Apple docs](https://developer.apple.com/documentation/coregraphics/1456395-cgdisplaybounds)
//! - macOS: `CGWindowListCreateImage` captures in physical pixels — [Apple docs](https://developer.apple.com/documentation/coregraphics/1454852-cgwindowlistcreateimage)
//! - macOS scale_factor: `CGDisplayMode::pixel_width() / CGDisplayBounds().width` (xcap `src/macos/impl_monitor.rs`)
//! - Windows: `DEVMODEW.dmPelsWidth` / `dmPelsHeight` = display resolution in physical pixels — [MSDN](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-devmodew)
//! - Linux: xcap divides raw XRandR dimensions by `Xft.dpi / 96` (xcap `src/linux/impl_monitor.rs` lines 305-324)
//! - `xcap` v0.9.3 — `Monitor::all()`, `Monitor::capture_image()`
//! - `tokio::task::spawn_blocking` — offloads synchronous work

use log::debug;
use xshot_domain::{Bounds, MonitorInfo, Screenshot, XshotError};
use xshot_utils::encode_rgba_to_png;

/// Converts xcap's reported geometry to physical pixels.
///
/// On **macOS**, `CGDisplayBounds()` returns logical points. The scale factor
/// is derived from `CGDisplayMode::pixel_width() / logical_width`, so
/// `logical × scale_factor = pixel_width` (exact for the primary axis).
///
/// On **Linux**, xcap divides the raw XRandR values by `scale_factor`,
/// producing logical coordinates. Multiplying back recovers the original
/// physical values (subject to minor rounding from the intermediate `as u32`
/// truncation in xcap).
///
/// On **Windows**, `DEVMODEW.dmPelsWidth` / `dmPelsHeight` already provide
/// physical pixel resolution, so no conversion is needed.
///
/// The `round()` call ensures we get the nearest integer when floating-point
/// arithmetic introduces sub-pixel drift.
#[cfg(not(target_os = "windows"))]
fn to_physical_dimensions(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
) -> (i32, i32, u32, u32) {
    (
        (f64::from(x) * scale_factor).round() as i32,
        (f64::from(y) * scale_factor).round() as i32,
        (f64::from(width) * scale_factor).round() as u32,
        (f64::from(height) * scale_factor).round() as u32,
    )
}

/// On Windows, xcap already reports physical pixels — passthrough.
#[cfg(target_os = "windows")]
fn to_physical_dimensions(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    _scale_factor: f64,
) -> (i32, i32, u32, u32) {
    (x, y, width, height)
}

/// Derives logical (DIP) coordinates from xcap's raw geometry.
///
/// On **macOS** and **Linux**, xcap already returns logical values, so the
/// raw dimensions are passed through unchanged.
///
/// On **Windows**, xcap returns physical pixels; dividing by the scale
/// factor produces the logical equivalents.
#[cfg(not(target_os = "windows"))]
fn to_logical_dimensions(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    _scale_factor: f64,
) -> (i32, i32, u32, u32) {
    (x, y, width, height)
}

/// On Windows, xcap reports physical pixels — derive logical by dividing.
#[cfg(target_os = "windows")]
fn to_logical_dimensions(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
) -> (i32, i32, u32, u32) {
    (
        (f64::from(x) / scale_factor).round() as i32,
        (f64::from(y) / scale_factor).round() as i32,
        (f64::from(width) / scale_factor).round() as u32,
        (f64::from(height) / scale_factor).round() as u32,
    )
}

/// Extracts normalised [`MonitorInfo`] from an `xcap::Monitor`.
///
/// Each accessor on `xcap::Monitor` returns `XCapResult<T>`. We map
/// failures to [`XshotError::ResourceUnavailable`] so the caller gets a
/// structured error rather than a raw xcap error string.
///
/// Dimensions (x, y, width, height) are normalised to **physical pixels**
/// on all platforms. See module-level documentation for details.
fn monitor_info(m: &xcap::Monitor) -> Result<MonitorInfo, XshotError> {
    let id = m
        .id()
        .map_err(|e| XshotError::resource_unavailable(format!("failed to read monitor id: {e}")))?;

    let name = m.name().map_err(|e| {
        XshotError::resource_unavailable(format!("failed to read monitor name: {e}"))
    })?;

    let friendly_name = m.friendly_name().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read friendly_name: {e}"))
    })?;

    // --- Scale factor (read early; needed for dimension normalisation) -------

    // xcap returns f32; widen to f64 for JavaScript interop (JS numbers are f64).
    let scale_factor = m
        .scale_factor()
        .map_err(|e| {
            XshotError::resource_unavailable(format!(
                "monitor {id}: failed to read scale_factor: {e}"
            ))
        })
        .map(f64::from)?;

    // --- Raw geometry (platform-dependent coordinate system) -----------------

    let raw_x = m.x().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read x: {e}"))
    })?;

    let raw_y = m.y().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read y: {e}"))
    })?;

    let raw_width = m.width().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read width: {e}"))
    })?;

    let raw_height = m.height().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read height: {e}"))
    })?;

    // --- Normalise to physical pixels ----------------------------------------

    let (x, y, width, height) =
        to_physical_dimensions(raw_x, raw_y, raw_width, raw_height, scale_factor);

    // --- Derive logical (DIP) coordinates ------------------------------------

    let (logical_x, logical_y, logical_width, logical_height) =
        to_logical_dimensions(raw_x, raw_y, raw_width, raw_height, scale_factor);

    // --- Remaining fields ----------------------------------------------------

    let rotation = m
        .rotation()
        .map_err(|e| {
            XshotError::resource_unavailable(format!("monitor {id}: failed to read rotation: {e}"))
        })
        .map(f64::from)?;

    let frequency = m
        .frequency()
        .map_err(|e| {
            XshotError::resource_unavailable(format!("monitor {id}: failed to read frequency: {e}"))
        })
        .map(f64::from)?;

    let is_primary = m.is_primary().map_err(|e| {
        XshotError::resource_unavailable(format!("monitor {id}: failed to read is_primary: {e}"))
    })?;

    // is_builtin may not be supported on all platforms; default to false.
    let is_builtin = m.is_builtin().unwrap_or(false);

    let physical = Bounds {
        x,
        y,
        width,
        height,
    };
    let logical = Bounds {
        x: logical_x,
        y: logical_y,
        width: logical_width,
        height: logical_height,
    };

    debug!(
        "monitor {id} ({name:?}): physical={}\u{00d7}{} logical={}\u{00d7}{} scale={scale_factor}",
        physical.width, physical.height, logical.width, logical.height,
    );

    Ok(MonitorInfo {
        id,
        name,
        friendly_name,
        physical,
        logical,
        rotation,
        scale_factor,
        frequency,
        is_primary,
        is_builtin,
    })
}

/// Returns normalised metadata for all connected monitors.
///
/// Runs on a blocking thread to avoid stalling the Tokio async runtime.
///
/// # Errors
///
/// - [`XshotError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`XshotError::InternalError`] if the blocking task panics.
pub async fn get_monitors() -> Result<Vec<MonitorInfo>, XshotError> {
    tokio::task::spawn_blocking(|| {
        let monitors = xcap::Monitor::all().map_err(|e| {
            XshotError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        let infos: Vec<MonitorInfo> = monitors
            .iter()
            .map(monitor_info)
            .collect::<Result<_, _>>()?;

        debug!("discovered {} monitor(s)", infos.len());
        Ok(infos)
    })
    .await
    .map_err(|e| XshotError::internal(format!("monitor listing task panicked: {e}")))?
}

/// Returns normalised metadata for the monitor with the given `id`.
///
/// # Errors
///
/// - [`XshotError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`XshotError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`XshotError::InternalError`] if the blocking task panics.
pub async fn get_monitor_by_id(id: u32) -> Result<MonitorInfo, XshotError> {
    tokio::task::spawn_blocking(move || {
        let monitors = xcap::Monitor::all().map_err(|e| {
            XshotError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        for m in &monitors {
            // id() itself is fallible; skip monitors whose id cannot be read.
            if let Ok(mid) = m.id()
                && mid == id
            {
                return monitor_info(m);
            }
        }

        Err(XshotError::monitor_not_found(id))
    })
    .await
    .map_err(|e| XshotError::internal(format!("monitor lookup task panicked: {e}")))?
}

/// Captures a PNG-encoded screenshot of the monitor with the given `id`.
///
/// The capture and PNG encoding both run on a blocking thread.
///
/// # Errors
///
/// - [`XshotError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`XshotError::CaptureFailed`] if the OS capture call fails.
/// - [`XshotError::EncodingError`] if PNG encoding fails.
/// - [`XshotError::InternalError`] if the blocking task panics.
pub async fn capture_monitor(id: u32) -> Result<Screenshot, XshotError> {
    tokio::task::spawn_blocking(move || {
        let monitors = xcap::Monitor::all().map_err(|e| {
            XshotError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        let xcap_monitor = monitors
            .iter()
            .find(|m| m.id().is_ok_and(|mid| mid == id))
            .ok_or_else(|| XshotError::monitor_not_found(id))?;

        let info = monitor_info(xcap_monitor)?;

        debug!(
            "capturing monitor {id} ({}\u{00d7}{})",
            info.physical.width, info.physical.height,
        );

        let image = xcap_monitor
            .capture_image()
            .map_err(|e| XshotError::capture_failed(format!("monitor {id}: {e}")))?;

        let data = encode_rgba_to_png(image.as_raw(), image.width(), image.height())?;

        debug!(
            "captured monitor {id}: {}\u{00d7}{} \u{2192} {} bytes PNG",
            image.width(),
            image.height(),
            data.len(),
        );

        Ok(Screenshot {
            monitor: info,
            data,
        })
    })
    .await
    .map_err(|e| XshotError::internal(format!("capture task panicked: {e}")))?
}

/// Captures PNG-encoded screenshots of **every** connected monitor.
///
/// Each monitor is captured sequentially on a single blocking thread to
/// minimise contention on the OS capture subsystem.
///
/// # Errors
///
/// - [`XshotError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`XshotError::CaptureFailed`] if any individual capture fails.
/// - [`XshotError::EncodingError`] if any PNG encoding fails.
/// - [`XshotError::InternalError`] if the blocking task panics.
pub async fn capture_all_monitors() -> Result<Vec<Screenshot>, XshotError> {
    tokio::task::spawn_blocking(|| {
        let monitors = xcap::Monitor::all().map_err(|e| {
            XshotError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        debug!("capturing all {} monitor(s)", monitors.len());

        let mut screenshots = Vec::with_capacity(monitors.len());

        for m in &monitors {
            let info = monitor_info(m)?;
            let mid = info.id;

            let image = m
                .capture_image()
                .map_err(|e| XshotError::capture_failed(format!("monitor {mid}: {e}")))?;

            let data = encode_rgba_to_png(image.as_raw(), image.width(), image.height())?;

            debug!(
                "captured monitor {mid}: {}\u{00d7}{} \u{2192} {} bytes PNG",
                image.width(),
                image.height(),
                data.len(),
            );

            screenshots.push(Screenshot {
                monitor: info,
                data,
            });
        }

        debug!("captured {} screenshot(s) total", screenshots.len());
        Ok(screenshots)
    })
    .await
    .map_err(|e| XshotError::internal(format!("capture-all task panicked: {e}")))?
}
