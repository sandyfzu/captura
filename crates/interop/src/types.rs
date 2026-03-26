//! NAPI-compatible types for the JavaScript API surface.
//!
//! These structs mirror the domain models ([`MonitorInfo`], [`Screenshot`])
//! but carry `#[napi(object)]` so that napi-rs can serialise them as plain
//! JavaScript objects. Conversion is done via `From` impls.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use xshot_domain::{
    Base64CaptureResult, Base64Screenshot, Bounds, CaptureResult, ImageFormat, MonitorInfo,
    Screenshot, Size,
};

/// A rectangular region in a 2D coordinate space.
///
/// Used to represent monitor geometry. The coordinate system (physical pixels
/// or logical DIP units) depends on context — see the `physical` and
/// `logical` fields on [`Monitor`].
#[napi(object, js_name = "Bounds")]
pub struct JsBounds {
    /// Horizontal position of the top-left corner.
    pub x: i32,
    /// Vertical position of the top-left corner.
    pub y: i32,
    /// Horizontal extent in the given coordinate system.
    pub width: u32,
    /// Vertical extent in the given coordinate system.
    pub height: u32,
}

impl From<Bounds> for JsBounds {
    fn from(b: Bounds) -> Self {
        Self {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }
    }
}

/// Monitor metadata.
///
/// A snapshot of monitor state at the time it was queried. This is a plain
/// data object — it does **not** hold a live reference to the OS resource.
/// Monitors can be disconnected or reconfigured at any time, so instances
/// may become stale. Use `captureMonitor(id)` or `getMonitorById(id)` to
/// re-query the OS for current state.
///
/// ## Physical vs. logical coordinates
///
/// `physical` contains geometry in **physical pixels** — values that always
/// match the dimensions of a captured screenshot buffer. This is what you
/// want when working with image data (drawing, saving, pixel manipulation).
///
/// `logical` contains the same geometry in **logical (DIP / CSS-point)
/// units** as the OS or window manager sees them. On a 2× Retina display a
/// 2560×1600 physical screen has 1280×800 logical dimensions. Physical and
/// logical values are identical on standard 1× displays.
///
/// Both are always populated. Convert between them with `scaleFactor`:
///
/// ```ts
/// physical = logical * scaleFactor
/// logical  = physical / scaleFactor
/// ```
///
/// ### Platform name fields
///
/// | Platform | `name` | `friendlyName` |
/// |----------|--------|----------------|
/// | macOS | `"Display #<model>"` (e.g. `"Display #16419"`) | `NSScreen.localizedName` (e.g. `"Built-in Retina Display"`) |
/// | Windows | Device path (e.g. `\\.\DISPLAY1`) | Monitor product name (e.g. `"LG UltraFine"`) |
/// | Linux | XRandR output (e.g. `"HDMI-1"`) | Same as `name` (X11 has no separate friendly name) |
#[napi(object, js_name = "Monitor")]
pub struct JsMonitor {
    /// OS-assigned monitor identifier. Stable within a single session but may
    /// change across reboots or when monitors are reconnected.
    pub id: u32,

    /// System-level device or output name.
    ///
    /// - **macOS**: `"Display #<model_number>"` (e.g. `"Display #16419"`)
    /// - **Windows**: device path (e.g. `\\.\DISPLAY1`)
    /// - **Linux**: XRandR output name (e.g. `"HDMI-1"`, `"eDP-1"`)
    pub name: String,

    /// Human-readable display name suitable for showing in a UI.
    ///
    /// - **macOS**: `NSScreen.localizedName` (e.g. `"Built-in Retina Display"`)
    /// - **Windows**: monitor product name from EDID (e.g. `"LG UltraFine"`)
    /// - **Linux**: same as `name` (X11 does not expose a separate friendly name)
    pub friendly_name: String,

    /// Monitor geometry in **physical pixels**. Values match the dimensions
    /// of captured screenshot buffers. Use this when working with image data.
    pub physical: JsBounds,

    /// Monitor geometry in **logical (DIP / CSS-point) units** as reported
    /// by the OS window manager. Use this for UI layout and positioning.
    pub logical: JsBounds,

    /// Display rotation in degrees. Common values: `0.0`, `90.0`, `180.0`,
    /// `270.0`.
    pub rotation: f64,

    /// HiDPI / Retina scale factor. `1.0` for standard displays, `2.0` for
    /// typical Retina or HiDPI configurations. Relates physical and logical
    /// coordinates: `physical = logical × scaleFactor`.
    pub scale_factor: f64,

    /// Display refresh rate in Hz (e.g. `60.0`, `120.0`, `144.0`).
    pub frequency: f64,

    /// Whether this is the primary (main) display as designated by the OS.
    pub is_primary: bool,

    /// Whether this is a built-in display (e.g. a laptop panel). Falls back
    /// to `false` on platforms that do not report this information.
    pub is_builtin: bool,
}

impl From<MonitorInfo> for JsMonitor {
    fn from(m: MonitorInfo) -> Self {
        Self {
            id: m.id,
            name: m.name,
            friendly_name: m.friendly_name,
            physical: JsBounds::from(m.physical),
            logical: JsBounds::from(m.logical),
            rotation: m.rotation,
            scale_factor: m.scale_factor,
            frequency: m.frequency,
            is_primary: m.is_primary,
            is_builtin: m.is_builtin,
        }
    }
}

/// Image dimensions in pixels.
///
/// Describes the extent of a captured image. For a full-monitor capture this
/// matches the monitor's `physical` bounds, but future region captures may
/// produce different dimensions.
#[napi(object, js_name = "Size")]
pub struct JsSize {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

impl From<Size> for JsSize {
    fn from(s: Size) -> Self {
        Self {
            width: s.width,
            height: s.height,
        }
    }
}

/// The encoding format of a captured screenshot.
///
/// Indicates which image codec was used to encode `Screenshot.data`.
/// All formats use default encoder settings. If you need fine-grained
/// control over encoding parameters (e.g. JPEG quality, AVIF speed),
/// capture as `"Png"` (lossless, pixel-perfect) and convert using your
/// preferred image processing library.
///
/// | Value | MIME type | Notes |
/// |-------|-----------|-------|
/// | `"Png"` | `image/png` | Default. Lossless, pixel-perfect. |
/// | `"Jpeg"` | `image/jpeg` | Lossy, default quality. |
/// | `"WebP"` | `image/webp` | Lossless only. |
/// | `"Avif"` | `image/avif` | Default speed and quality. |
#[napi(string_enum, js_name = "ImageFormat")]
pub enum JsImageFormat {
    /// PNG — lossless, pixel-perfect. Default format.
    Png,
    /// JPEG — lossy compression, default quality.
    Jpeg,
    /// WebP — lossless encoding only.
    WebP,
    /// AVIF — default speed and quality settings.
    Avif,
}

impl From<ImageFormat> for JsImageFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Png => Self::Png,
            ImageFormat::Jpeg => Self::Jpeg,
            ImageFormat::WebP => Self::WebP,
            ImageFormat::Avif => Self::Avif,
        }
    }
}

impl From<JsImageFormat> for ImageFormat {
    fn from(f: JsImageFormat) -> Self {
        match f {
            JsImageFormat::Png => Self::Png,
            JsImageFormat::Jpeg => Self::Jpeg,
            JsImageFormat::WebP => Self::WebP,
            JsImageFormat::Avif => Self::Avif,
        }
    }
}

/// A captured screenshot — the image payload with its dimensions and format.
///
/// `data` contains encoded image bytes in the format indicated by `format`
/// (PNG by default). It can be written to disk, served over HTTP, or passed
/// directly to any image library without additional processing.
///
/// All formats use default encoder settings:
///
/// - **PNG**: lossless, default compression.
/// - **JPEG**: lossy, default quality.
/// - **WebP**: lossless encoding only.
/// - **AVIF**: default speed and quality.
///
/// If you need custom encoding parameters, capture as PNG (lossless) and
/// re-encode with your preferred image processing library.
///
/// `size` reflects the **actual** pixel dimensions of the encoded image.
/// Use `size.width` and `size.height` to know the image dimensions without
/// inspecting the encoded bytes.
///
/// `format` tells you which codec was used, so you can set the correct
/// `Content-Type` header or file extension without guessing.
#[napi(object, js_name = "Screenshot")]
pub struct JsScreenshot {
    /// Actual pixel dimensions of the encoded image.
    pub size: JsSize,
    /// The encoding format of `data` (e.g. `"Png"`).
    pub format: JsImageFormat,
    /// Encoded image bytes in the format specified by `format`.
    pub data: Buffer,
}

impl From<Screenshot> for JsScreenshot {
    fn from(s: Screenshot) -> Self {
        Self {
            size: JsSize::from(s.size),
            format: JsImageFormat::from(s.format),
            data: s.data.into(),
        }
    }
}

/// The result of a capture operation — pairs monitor metadata with the
/// captured screenshot.
///
/// Returned by `captureMonitor()` and `captureAllMonitors()`.
///
/// ```ts
/// const result: CaptureResult = await captureMonitor(1)
/// result.monitor.name            // "Built-in Retina Display"
/// result.screenshot.size.width   // 2560
/// result.screenshot.data         // <Buffer 89 50 4e 47 ...>
/// ```
#[napi(object, js_name = "CaptureResult")]
pub struct JsCaptureResult {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: JsMonitor,
    /// The captured image with its dimensions and encoded bytes.
    pub screenshot: JsScreenshot,
}

impl From<CaptureResult> for JsCaptureResult {
    fn from(r: CaptureResult) -> Self {
        Self {
            monitor: JsMonitor::from(r.monitor),
            screenshot: JsScreenshot::from(r.screenshot),
        }
    }
}

/// A captured screenshot with Base64-encoded data.
///
/// Identical to [`JsScreenshot`] except `data` is a
/// [RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// Base64 string instead of a `Buffer`. Useful when the consumer needs a
/// string representation — for example, embedding in JSON payloads, data
/// URIs, or HTML `<img>` tags:
///
/// ```ts
/// const { screenshot } = await captureMonitorBase64(1)
/// const dataUri = `data:image/png;base64,${screenshot.data}`
/// ```
#[napi(object, js_name = "Base64Screenshot")]
pub struct JsBase64Screenshot {
    /// Actual pixel dimensions of the encoded image.
    pub size: JsSize,
    /// The encoding format of the image before Base64 encoding.
    pub format: JsImageFormat,
    /// Base64-encoded image data (RFC 4648 standard alphabet with padding).
    pub data: String,
}

impl From<Base64Screenshot> for JsBase64Screenshot {
    fn from(s: Base64Screenshot) -> Self {
        Self {
            size: JsSize::from(s.size),
            format: JsImageFormat::from(s.format),
            data: s.data,
        }
    }
}

/// The result of a capture-to-Base64 operation — pairs monitor metadata
/// with a Base64-encoded screenshot.
///
/// Returned by `captureMonitorBase64()` and `captureAllMonitorsBase64()`.
///
/// ```ts
/// const result: Base64CaptureResult = await captureMonitorBase64(1)
/// result.monitor.name              // "Built-in Retina Display"
/// result.screenshot.size.width     // 2560
/// result.screenshot.data           // "iVBORw0KGgo..."
/// ```
#[napi(object, js_name = "Base64CaptureResult")]
pub struct JsBase64CaptureResult {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: JsMonitor,
    /// The captured image with Base64-encoded data.
    pub screenshot: JsBase64Screenshot,
}

impl From<Base64CaptureResult> for JsBase64CaptureResult {
    fn from(r: Base64CaptureResult) -> Self {
        Self {
            monitor: JsMonitor::from(r.monitor),
            screenshot: JsBase64Screenshot::from(r.screenshot),
        }
    }
}
