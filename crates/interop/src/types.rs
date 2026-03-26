//! NAPI-compatible types for the JavaScript API surface.
//!
//! These structs mirror the domain models ([`MonitorInfo`], [`Screenshot`])
//! but carry `#[napi(object)]` so that napi-rs can serialise them as plain
//! JavaScript objects. Conversion is done via `From` impls.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use xshot_domain::{Bounds, MonitorInfo, Screenshot};

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
    /// `physical.width` × `physical.height`.
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
