//! NAPI-compatible types for the JavaScript API surface.
//!
//! These structs mirror the domain models ([`MonitorInfo`], [`Screenshot`])
//! but carry `#[napi(object)]` so that napi-rs can serialise them as plain
//! JavaScript objects. Conversion is done via `From` impls.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use xshot_domain::{MonitorInfo, Screenshot};

/// JavaScript-facing monitor metadata.
///
/// A snapshot of monitor state at the time it was queried. This is a plain
/// data object — it does **not** hold a live reference to the OS resource.
/// Monitors can be disconnected or reconfigured at any time, so instances
/// may become stale. Use `captureMonitor(id)` or `getMonitorById(id)` to
/// re-query the OS for current state.
///
/// ## Physical vs. logical coordinates
///
/// The top-level geometry fields (`x`, `y`, `width`, `height`) are in
/// **physical pixels** — they always match the dimensions of a captured
/// screenshot buffer. This is the default because screenshot consumers
/// (image libraries, canvas drawing, file output) operate in physical pixels.
///
/// The `logical*` fields (`logicalX`, `logicalY`, `logicalWidth`,
/// `logicalHeight`) represent the same geometry in **logical (DIP / CSS-point)
/// units** as the OS or window manager sees them. On a 2× Retina display a
/// 2560×1600 physical screen has 1280×800 logical dimensions.
/// Physical and logical values will be the same on a standard 1× display, but on HiDPI / Retina screens they differ by the `scaleFactor`.
///
/// Both sets are always populated. You can convert between them with the
/// `scaleFactor`:
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

    /// Horizontal position of the monitor's top-left corner in **physical
    /// pixels** within the virtual-screen coordinate space. This is the
    /// default coordinate system — it matches captured screenshot dimensions.
    pub x: i32,

    /// Vertical position of the monitor's top-left corner in **physical
    /// pixels** within the virtual-screen coordinate space.
    pub y: i32,

    /// Horizontal resolution in **physical pixels**. Matches the width of
    /// a captured screenshot buffer for this monitor.
    pub width: u32,

    /// Vertical resolution in **physical pixels**. Matches the height of
    /// a captured screenshot buffer for this monitor.
    pub height: u32,

    /// Horizontal position of the monitor's top-left corner in **logical
    /// (DIP / CSS-point) units**. Equal to `x / scaleFactor`.
    pub logical_x: i32,

    /// Vertical position of the monitor's top-left corner in **logical
    /// (DIP / CSS-point) units**. Equal to `y / scaleFactor`.
    pub logical_y: i32,

    /// Horizontal resolution in **logical (DIP / CSS-point) units**.
    /// Equal to `width / scaleFactor`.
    pub logical_width: u32,

    /// Vertical resolution in **logical (DIP / CSS-point) units**.
    /// Equal to `height / scaleFactor`.
    pub logical_height: u32,

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
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
            logical_x: m.logical_x,
            logical_y: m.logical_y,
            logical_width: m.logical_width,
            logical_height: m.logical_height,
            rotation: m.rotation,
            scale_factor: m.scale_factor,
            frequency: m.frequency,
            is_primary: m.is_primary,
            is_builtin: m.is_builtin,
        }
    }
}

/// JavaScript-facing screenshot result pairing monitor metadata with the
/// captured image data.
///
/// The `data` buffer contains a **PNG-encoded** image by default. It can be
/// written to disk, served over HTTP, or passed directly to any image
/// library without additional processing.
#[napi(object, js_name = "Screenshot")]
pub struct JsScreenshot {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: JsMonitor,
    /// PNG-encoded image bytes. The buffer dimensions match the monitor's
    /// physical `width` × `height`.
    pub data: Buffer,
}

impl From<Screenshot> for JsScreenshot {
    fn from(s: Screenshot) -> Self {
        Self {
            monitor: JsMonitor::from(s.monitor),
            data: s.data.into(),
        }
    }
}
