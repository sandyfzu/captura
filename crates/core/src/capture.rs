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

use captura_domain::{
    Base64CaptureResult, Base64Screenshot, Bounds, CapturaError, CaptureResult, ImageFormat,
    MonitorInfo, Screenshot, Size,
};
use captura_utils::{encode_rgba, encode_rgba_base64};
use log::debug;
use xcap::image::RgbaImage;

// ---------------------------------------------------------------------------
// Internal raw-capture types
// ---------------------------------------------------------------------------

/// The raw result of a single monitor capture — monitor metadata paired with
/// the unencoded RGBA pixel buffer.
///
/// This is an internal type that never leaves the core layer. Public
/// functions consume it and apply the requested encoding (binary or Base64)
/// before returning a domain result type.
struct RawCapture {
    info: MonitorInfo,
    image: RgbaImage,
}

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
/// failures to [`CapturaError::ResourceUnavailable`] so the caller gets a
/// structured error rather than a raw xcap error string.
///
/// Dimensions (x, y, width, height) are normalised to **physical pixels**
/// on all platforms. See module-level documentation for details.
fn monitor_info(m: &xcap::Monitor) -> Result<MonitorInfo, CapturaError> {
    let id = m.id().map_err(|e| {
        CapturaError::resource_unavailable(format!("failed to read monitor id: {e}"))
    })?;

    let name = m.name().map_err(|e| {
        CapturaError::resource_unavailable(format!("failed to read monitor name: {e}"))
    })?;

    let friendly_name = m.friendly_name().map_err(|e| {
        CapturaError::resource_unavailable(format!(
            "monitor {id}: failed to read friendly_name: {e}"
        ))
    })?;

    // --- Scale factor (read early; needed for dimension normalisation) -------

    // xcap returns f32; widen to f64 for JavaScript interop (JS numbers are f64).
    let scale_factor = m
        .scale_factor()
        .map_err(|e| {
            CapturaError::resource_unavailable(format!(
                "monitor {id}: failed to read scale_factor: {e}"
            ))
        })
        .map(f64::from)?;

    // --- Raw geometry (platform-dependent coordinate system) -----------------

    let raw_x = m.x().map_err(|e| {
        CapturaError::resource_unavailable(format!("monitor {id}: failed to read x: {e}"))
    })?;

    let raw_y = m.y().map_err(|e| {
        CapturaError::resource_unavailable(format!("monitor {id}: failed to read y: {e}"))
    })?;

    let raw_width = m.width().map_err(|e| {
        CapturaError::resource_unavailable(format!("monitor {id}: failed to read width: {e}"))
    })?;

    let raw_height = m.height().map_err(|e| {
        CapturaError::resource_unavailable(format!("monitor {id}: failed to read height: {e}"))
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
            CapturaError::resource_unavailable(format!(
                "monitor {id}: failed to read rotation: {e}"
            ))
        })
        .map(f64::from)?;

    let frequency = m
        .frequency()
        .map_err(|e| {
            CapturaError::resource_unavailable(format!(
                "monitor {id}: failed to read frequency: {e}"
            ))
        })
        .map(f64::from)?;

    let is_primary = m.is_primary().map_err(|e| {
        CapturaError::resource_unavailable(format!("monitor {id}: failed to read is_primary: {e}"))
    })?;

    // is_builtin may not be supported on all platforms; default to false.
    let is_builtin = m.is_builtin().unwrap_or_else(|e| {
        debug!("monitor {id}: is_builtin unavailable, defaulting to false: {e}");
        false
    });

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
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn get_monitors() -> Result<Vec<MonitorInfo>, CapturaError> {
    tokio::task::spawn_blocking(|| {
        let monitors = xcap::Monitor::all().map_err(|e| {
            CapturaError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        let infos: Vec<MonitorInfo> = monitors
            .iter()
            .map(monitor_info)
            .collect::<Result<_, _>>()?;

        debug!("discovered {} monitor(s)", infos.len());
        Ok(infos)
    })
    .await
    .map_err(|e| CapturaError::internal(format!("monitor listing task panicked: {e}")))?
}

/// Returns normalised metadata for the monitor with the given `id`.
///
/// # Errors
///
/// - [`CapturaError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn get_monitor_by_id(id: u32) -> Result<MonitorInfo, CapturaError> {
    tokio::task::spawn_blocking(move || {
        let monitors = xcap::Monitor::all().map_err(|e| {
            CapturaError::resource_unavailable(format!("failed to list monitors: {e}"))
        })?;

        for m in &monitors {
            // id() itself is fallible; skip monitors whose id cannot be read.
            if let Ok(mid) = m.id()
                && mid == id
            {
                return monitor_info(m);
            }
        }

        Err(CapturaError::monitor_not_found(id))
    })
    .await
    .map_err(|e| CapturaError::internal(format!("monitor lookup task panicked: {e}")))?
}

// ---------------------------------------------------------------------------
// Internal raw-capture helpers
// ---------------------------------------------------------------------------

/// Finds the monitor with the given `id`, captures its screen contents, and
/// returns the raw RGBA pixel buffer alongside normalised metadata.
///
/// This is the single point where OS monitor lookup + capture happens for
/// single-monitor operations. Public functions call this and then apply the
/// requested encoding.
///
/// # Errors
///
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`CapturaError::CaptureFailed`] if the OS capture call fails.
fn raw_capture_monitor(id: u32) -> Result<RawCapture, CapturaError> {
    let monitors = xcap::Monitor::all()
        .map_err(|e| CapturaError::resource_unavailable(format!("failed to list monitors: {e}")))?;

    let xcap_monitor = monitors
        .iter()
        .find(|m| m.id().is_ok_and(|mid| mid == id))
        .ok_or_else(|| CapturaError::monitor_not_found(id))?;

    let info = monitor_info(xcap_monitor)?;

    debug!(
        "capturing monitor {id} ({}\u{00d7}{})",
        info.physical.width, info.physical.height,
    );

    let image = xcap_monitor
        .capture_image()
        .map_err(|e| CapturaError::capture_failed(format!("monitor {id}: {e}")))?;

    Ok(RawCapture { info, image })
}

/// Captures the screen contents of every connected monitor and returns the
/// raw RGBA pixel buffers alongside normalised metadata.
///
/// This is the single point where OS monitor enumeration + capture happens
/// for multi-monitor operations. Public functions call this and then apply
/// the requested encoding to each result.
///
/// # Errors
///
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::CaptureFailed`] if any individual capture fails.
fn raw_capture_all_monitors() -> Result<Vec<RawCapture>, CapturaError> {
    let monitors = xcap::Monitor::all()
        .map_err(|e| CapturaError::resource_unavailable(format!("failed to list monitors: {e}")))?;

    debug!("capturing all {} monitor(s)", monitors.len());

    let mut results = Vec::with_capacity(monitors.len());

    for m in &monitors {
        let info = monitor_info(m)?;
        let mid = info.id;

        let image = m
            .capture_image()
            .map_err(|e| CapturaError::capture_failed(format!("monitor {mid}: {e}")))?;

        results.push(RawCapture { info, image });
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Internal encoding helpers
// ---------------------------------------------------------------------------

/// Encodes a [`RawCapture`] into a [`CaptureResult`] with binary image data.
///
/// When `format` is [`ImageFormat::Raw`], the RGBA8 pixel buffer is moved
/// directly into the result via [`RgbaImage::into_raw()`] — zero encoding,
/// zero copies.  For all other formats the pixels are encoded by the
/// utility layer.
fn encode_capture(raw: RawCapture, format: ImageFormat) -> Result<CaptureResult, CapturaError> {
    let width = raw.image.width();
    let height = raw.image.height();

    let data = if format == ImageFormat::Raw {
        raw.image.into_raw()
    } else {
        encode_rgba(raw.image.as_raw(), width, height, format)?
    };

    debug!(
        "encoded monitor {}: {width}\u{00d7}{height} \u{2192} {} bytes {format}",
        raw.info.id,
        data.len(),
    );

    Ok(CaptureResult {
        monitor: raw.info,
        screenshot: Screenshot {
            size: Size { width, height },
            format,
            data,
        },
    })
}

/// Encodes a [`RawCapture`] into a [`Base64CaptureResult`] with a Base64
/// string.
fn encode_capture_base64(
    raw: RawCapture,
    format: ImageFormat,
) -> Result<Base64CaptureResult, CapturaError> {
    let data = encode_rgba_base64(
        raw.image.as_raw(),
        raw.image.width(),
        raw.image.height(),
        format,
    )?;

    debug!(
        "encoded monitor {} (base64): {}\u{00d7}{} \u{2192} {} chars {format}",
        raw.info.id,
        raw.image.width(),
        raw.image.height(),
        data.len(),
    );

    Ok(Base64CaptureResult {
        monitor: raw.info,
        screenshot: Base64Screenshot {
            size: Size {
                width: raw.image.width(),
                height: raw.image.height(),
            },
            format,
            data,
        },
    })
}

// ---------------------------------------------------------------------------
// Public capture API
// ---------------------------------------------------------------------------

/// Captures an encoded screenshot of the monitor with the given `id`.
///
/// The capture and image encoding both run on a blocking thread.
///
/// # Arguments
///
/// * `id` — OS-assigned monitor identifier.
/// * `format` — Target encoding format (e.g. Raw, PNG, JPEG, WebP, AVIF).
///   When `Raw`, the RGBA8 pixel buffer is returned without encoding.
///
/// # Errors
///
/// - [`CapturaError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`CapturaError::CaptureFailed`] if the OS capture call fails.
/// - [`CapturaError::EncodingError`] if image encoding fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn capture_monitor(id: u32, format: ImageFormat) -> Result<CaptureResult, CapturaError> {
    tokio::task::spawn_blocking(move || {
        let raw = raw_capture_monitor(id)?;
        encode_capture(raw, format)
    })
    .await
    .map_err(|e| CapturaError::internal(format!("capture task panicked: {e}")))?
}

/// Captures encoded screenshots of **every** connected monitor.
///
/// Each monitor is captured sequentially on a single blocking thread to
/// minimise contention on the OS capture subsystem.
///
/// # Arguments
///
/// * `format` — Target encoding format applied to all captures. When `Raw`,
///   the RGBA8 pixel buffers are returned without encoding.
///
/// # Errors
///
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::CaptureFailed`] if any individual capture fails.
/// - [`CapturaError::EncodingError`] if image encoding fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn capture_all_monitors(format: ImageFormat) -> Result<Vec<CaptureResult>, CapturaError> {
    tokio::task::spawn_blocking(move || {
        let raws = raw_capture_all_monitors()?;
        let count = raws.len();
        let results = raws
            .into_iter()
            .map(|raw| encode_capture(raw, format))
            .collect::<Result<Vec<_>, _>>()?;
        debug!("captured {count} screenshot(s) total");
        Ok(results)
    })
    .await
    .map_err(|e| CapturaError::internal(format!("capture-all task panicked: {e}")))?
}

/// Captures a screenshot of the monitor with the given `id` and returns
/// the image data as a Base64-encoded string.
///
/// Identical to [`capture_monitor`] except the screenshot data is a
/// [RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// Base64 string instead of raw bytes.
///
/// # Arguments
///
/// * `id` — OS-assigned monitor identifier.
/// * `format` — Target encoding format (e.g. PNG, JPEG, WebP, AVIF).
///
/// # Errors
///
/// - [`CapturaError::MonitorNotFound`] if no monitor matches the `id`.
/// - [`CapturaError::CaptureFailed`] if the OS capture call fails.
/// - [`CapturaError::EncodingError`] if image encoding fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn capture_monitor_base64(
    id: u32,
    format: ImageFormat,
) -> Result<Base64CaptureResult, CapturaError> {
    tokio::task::spawn_blocking(move || {
        let raw = raw_capture_monitor(id)?;
        encode_capture_base64(raw, format)
    })
    .await
    .map_err(|e| CapturaError::internal(format!("base64 capture task panicked: {e}")))?
}

/// Captures Base64-encoded screenshots of **every** connected monitor.
///
/// Identical to [`capture_all_monitors`] except each screenshot's data is a
/// [RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// Base64 string instead of raw bytes.
///
/// # Arguments
///
/// * `format` — Target encoding format applied to all captures.
///
/// # Errors
///
/// - [`CapturaError::ResourceUnavailable`] if `xcap::Monitor::all()` fails.
/// - [`CapturaError::CaptureFailed`] if any individual capture fails.
/// - [`CapturaError::EncodingError`] if image encoding fails.
/// - [`CapturaError::InternalError`] if the blocking task panics.
pub async fn capture_all_monitors_base64(
    format: ImageFormat,
) -> Result<Vec<Base64CaptureResult>, CapturaError> {
    tokio::task::spawn_blocking(move || {
        let raws = raw_capture_all_monitors()?;
        let count = raws.len();
        let results = raws
            .into_iter()
            .map(|raw| encode_capture_base64(raw, format))
            .collect::<Result<Vec<_>, _>>()?;
        debug!("captured {count} base64 screenshot(s) total");
        Ok(results)
    })
    .await
    .map_err(|e| CapturaError::internal(format!("base64 capture-all task panicked: {e}")))?
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    // -- Coordinate normalisation (pure math) --------------------------------
    //
    // These test `to_physical_dimensions` and `to_logical_dimensions`, which
    // are the core of the platform-normalisation logic.
    //
    // On macOS/Linux (the `#[cfg(not(target_os = "windows"))]` path):
    //   physical = raw × scale_factor
    //   logical  = raw  (passthrough)
    //
    // On Windows (`#[cfg(target_os = "windows")]` path):
    //   physical = raw  (passthrough)
    //   logical  = raw ÷ scale_factor
    //
    // The active path depends on the build target, so these tests verify
    // whichever variant is compiled.

    #[test]
    fn physical_dims_scale_1x() {
        // At 1× scale, physical should equal raw on all platforms.
        let (x, y, w, h) = to_physical_dimensions(100, 200, 1920, 1080, 1.0);
        assert_eq!((x, y, w, h), (100, 200, 1920, 1080));
    }

    #[test]
    fn logical_dims_scale_1x() {
        // At 1× scale, logical should equal raw on all platforms.
        let (x, y, w, h) = to_logical_dimensions(100, 200, 1920, 1080, 1.0);
        assert_eq!((x, y, w, h), (100, 200, 1920, 1080));
    }

    #[cfg(not(target_os = "windows"))]
    mod non_windows {
        use super::*;

        #[test]
        fn physical_dims_scale_2x() {
            // macOS Retina / Linux HiDPI: raw is logical, physical = raw × 2.
            let (x, y, w, h) = to_physical_dimensions(0, 0, 1280, 800, 2.0);
            assert_eq!((x, y, w, h), (0, 0, 2560, 1600));
        }

        #[test]
        fn physical_dims_with_offset() {
            // Verify position is also scaled.
            let (x, y, w, h) = to_physical_dimensions(100, 50, 1920, 1080, 2.0);
            assert_eq!((x, y, w, h), (200, 100, 3840, 2160));
        }

        #[test]
        fn physical_dims_fractional_scale() {
            // 1.5× scale → verify rounding (uses round()).
            // 1920 × 1.5 = 2880 (exact), 1080 × 1.5 = 1620 (exact).
            let (_, _, w, h) = to_physical_dimensions(0, 0, 1920, 1080, 1.5);
            assert_eq!((w, h), (2880, 1620));
        }

        #[test]
        fn physical_dims_rounds_half_up() {
            // 101 × 1.5 = 151.5 → rounds to 152 (round-half-to-even in f64,
            // but .round() uses "round half away from zero").
            // Source: https://doc.rust-lang.org/std/primitive.f64.html#method.round
            let (_, _, w, _) = to_physical_dimensions(0, 0, 101, 1, 1.5);
            assert_eq!(w, 152); // 151.5.round() = 152
        }

        #[test]
        fn physical_dims_negative_position() {
            // Multi-monitor setups can have negative coordinates.
            let (x, y, _, _) = to_physical_dimensions(-1920, -100, 1920, 1080, 2.0);
            assert_eq!((x, y), (-3840, -200));
        }

        #[test]
        fn logical_dims_passthrough() {
            // On macOS/Linux, logical = raw (no transformation).
            let (x, y, w, h) = to_logical_dimensions(-50, 100, 2560, 1440, 2.0);
            assert_eq!((x, y, w, h), (-50, 100, 2560, 1440));
        }
    }

    #[cfg(target_os = "windows")]
    mod windows {
        use super::*;

        #[test]
        fn physical_dims_passthrough() {
            // On Windows, physical = raw (no transformation).
            let (x, y, w, h) = to_physical_dimensions(0, 0, 2560, 1440, 2.0);
            assert_eq!((x, y, w, h), (0, 0, 2560, 1440));
        }

        #[test]
        fn logical_dims_scale_2x() {
            // Windows: raw is physical, logical = raw ÷ 2.
            let (x, y, w, h) = to_logical_dimensions(0, 0, 2560, 1600, 2.0);
            assert_eq!((x, y, w, h), (0, 0, 1280, 800));
        }

        #[test]
        fn logical_dims_negative_position() {
            let (x, y, _, _) = to_logical_dimensions(-3840, -200, 1920, 1080, 2.0);
            assert_eq!((x, y), (-1920, -100));
        }
    }

    // -- encode_capture (binary encoding helpers) ----------------------------

    /// Helper: build a synthetic `RawCapture` with a 2×2 red RGBA image.
    fn make_raw_capture(id: u32, width: u32, height: u32) -> RawCapture {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[255, 0, 0, 255]); // opaque red
        }
        let image =
            RgbaImage::from_raw(width, height, pixels).expect("test image should be constructible");
        RawCapture {
            info: MonitorInfo {
                id,
                name: format!("test-monitor-{id}"),
                friendly_name: format!("Test Monitor {id}"),
                physical: Bounds {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                logical: Bounds {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                rotation: 0.0,
                scale_factor: 1.0,
                frequency: 60.0,
                is_primary: true,
                is_builtin: false,
            },
            image,
        }
    }

    #[test]
    fn encode_capture_raw_bypasses_encoding() {
        let raw = make_raw_capture(1, 2, 2);
        let result = encode_capture(raw, ImageFormat::Raw).expect("Raw encoding should succeed");
        // Raw returns the pixel buffer directly: 2×2×4 = 16 bytes.
        assert_eq!(result.screenshot.data.len(), 16);
        assert_eq!(result.screenshot.format, ImageFormat::Raw);
        assert_eq!(result.screenshot.size.width, 2);
        assert_eq!(result.screenshot.size.height, 2);
        // First pixel should be opaque red.
        assert_eq!(&result.screenshot.data[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn encode_capture_png_produces_valid_png() {
        let raw = make_raw_capture(1, 2, 2);
        let result = encode_capture(raw, ImageFormat::Png).expect("PNG encoding should succeed");
        assert_eq!(result.screenshot.format, ImageFormat::Png);
        // PNG magic bytes.
        assert!(
            result
                .screenshot
                .data
                .starts_with(&[0x89, b'P', b'N', b'G'])
        );
    }

    #[test]
    fn encode_capture_preserves_monitor_info() {
        let raw = make_raw_capture(42, 4, 4);
        let result = encode_capture(raw, ImageFormat::Raw).expect("Raw should succeed");
        assert_eq!(result.monitor.id, 42);
        assert_eq!(result.monitor.name, "test-monitor-42");
        assert!(result.monitor.is_primary);
    }

    #[test]
    fn encode_capture_base64_produces_valid_base64() {
        let raw = make_raw_capture(1, 2, 2);
        let result =
            encode_capture_base64(raw, ImageFormat::Png).expect("Base64 PNG should succeed");
        assert_eq!(result.screenshot.format, ImageFormat::Png);
        // Verify it's valid base64 that decodes to PNG.
        let decoded = base64::prelude::BASE64_STANDARD
            .decode(&result.screenshot.data)
            .expect("output should be valid base64");
        assert!(decoded.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn encode_capture_base64_preserves_dimensions() {
        let raw = make_raw_capture(1, 4, 3);
        let result =
            encode_capture_base64(raw, ImageFormat::Jpeg).expect("Base64 JPEG should succeed");
        assert_eq!(result.screenshot.size.width, 4);
        assert_eq!(result.screenshot.size.height, 3);
    }
}
