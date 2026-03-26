//! Monitor and screenshot domain models.
//!
//! These types represent the normalised view of a monitor and its captured
//! image data. They are transport-agnostic — the interop layer maps them to
//! NAPI-compatible types before returning them to JavaScript.
//!
//! # Design rationale — monitors as plain data
//!
//! [`MonitorInfo`] is a snapshot of monitor metadata at the moment it was
//! queried. It intentionally does **not** carry capture methods or hold a
//! reference to the underlying OS resource. Monitors can be disconnected or
//! reconfigured at any time — a cached `MonitorInfo` instance may become
//! stale. The capture API (`capture_monitor(id)`) re-queries the OS on
//! every call, ensuring it always operates on the current hardware state.
//! This keeps the data model simple and avoids subtle bugs from holding
//! stale handles.

/// A rectangular region in a 2D coordinate space.
///
/// Used to represent monitor geometry in both physical-pixel and logical
/// (DIP / CSS-point) coordinate systems. See [`MonitorInfo::physical`] and
/// [`MonitorInfo::logical`] for the two coordinate spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Horizontal position of the top-left corner.
    pub x: i32,
    /// Vertical position of the top-left corner.
    pub y: i32,
    /// Horizontal extent.
    pub width: u32,
    /// Vertical extent.
    pub height: u32,
}

/// Normalised metadata for a single display monitor.
///
/// Geometry is exposed as two [`Bounds`] structs:
///
/// - **`physical`**: pixel-exact dimensions that match captured screenshot
///   buffers on every platform.
/// - **`logical`**: DIP / CSS-point dimensions as the OS window manager
///   sees them. On a 2× Retina display a 2560×1600 physical screen has
///   1280×800 logical dimensions.
///
/// Both are always populated. On platforms where xcap already reports
/// physical pixels (Windows), logical values are derived by dividing by the
/// scale factor. On platforms where xcap reports logical values (macOS,
/// Linux), physical values are derived by multiplying by the scale factor.
///
/// You can convert between them:
///
/// ```text
/// physical = logical × scale_factor
/// logical  = physical ÷ scale_factor
/// ```
///
/// # Field semantics
///
/// | Field | Description |
/// |---|---|
/// | `id` | OS-assigned monitor identifier (stable within a session) |
/// | `name` | System-level device / output name (see *Platform names* below) |
/// | `friendly_name` | Human-readable display name (see *Platform names* below) |
/// | `physical` | Geometry in **physical pixels** (matches screenshot dimensions) |
/// | `logical` | Geometry in **logical / DIP** units |
/// | `rotation` | Display rotation in degrees (0.0, 90.0, …) |
/// | `scale_factor` | HiDPI / Retina scale (1.0 = standard, 2.0 = Retina) |
/// | `frequency` | Refresh rate in Hz |
/// | `is_primary` | Whether this is the primary / main display |
/// | `is_builtin` | Whether this is a built-in display (e.g. laptop panel) |
///
/// # Platform names
///
/// | Platform | `name` | `friendly_name` |
/// |---|---|---|
/// | macOS | `"Display #<model_number>"` (e.g. `"Display #16419"`) | `NSScreen.localizedName` (e.g. `"Built-in Retina Display"`) |
/// | Windows | Device path (e.g. `\\.\DISPLAY1`) | Monitor product name (e.g. `"LG UltraFine"`) |
/// | Linux | XRandR output name (e.g. `"HDMI-1"`) | Same as `name` (no separate friendly name on X11) |
///
/// # Platform coordinate sources
///
/// | Platform | xcap raw coordinates | Physical | Logical |
/// |---|---|---|---|
/// | macOS | `CGDisplayBounds` (logical) | raw × scale_factor | raw (passthrough) |
/// | Linux | XRandR raw ÷ scale_factor (logical) | raw × scale_factor | raw (passthrough) |
/// | Windows | `DEVMODEW` (physical) | raw (passthrough) | raw ÷ scale_factor |
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub friendly_name: String,
    /// Geometry in **physical pixels** — matches captured screenshot dimensions.
    pub physical: Bounds,
    /// Geometry in **logical (DIP / CSS-point) units**.
    pub logical: Bounds,
    pub rotation: f64,
    pub scale_factor: f64,
    pub frequency: f64,
    pub is_primary: bool,
    pub is_builtin: bool,
}

/// A captured screenshot paired with its source monitor metadata.
///
/// `data` contains encoded image bytes (PNG by default). The encoding is
/// performed in the utility layer before this struct is constructed.
#[derive(Debug, Clone)]
pub struct Screenshot {
    /// The monitor from which this screenshot was taken.
    pub monitor: MonitorInfo,
    /// Encoded image bytes (e.g. PNG).
    pub data: Vec<u8>,
}
