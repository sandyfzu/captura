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

/// Dimensions of an image in pixels.
///
/// Unlike [`Bounds`], this carries no position — it describes the *extent*
/// of an image, not its placement within a coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// The encoding format of a captured screenshot.
///
/// Indicates how `Screenshot::data` is encoded. This is a closed set of
/// formats that xshot supports — all backed by the [`image`] crate.
///
/// All formats use default encoder settings. For fine-grained control over
/// encoding parameters (e.g. JPEG quality, AVIF speed), capture as PNG
/// (lossless) and re-encode externally.
///
/// # Supported formats
///
/// | Variant | MIME type | Notes |
/// |---------|-----------|-------|
/// | `Png` | `image/png` | Default. Lossless, best for pixel-perfect captures. |
/// | `Jpeg` | `image/jpeg` | Lossy. Default quality (75). |
/// | `WebP` | `image/webp` | Lossless only (the `image` crate does not support lossy WebP). |
/// | `Avif` | `image/avif` | Default speed and quality settings. |
///
/// # Sources
///
/// - PNG: [`image::codecs::png::PngEncoder`](https://docs.rs/image/0.25/image/codecs/png/struct.PngEncoder.html)
/// - JPEG: [`image::codecs::jpeg::JpegEncoder`](https://docs.rs/image/0.25/image/codecs/jpeg/struct.JpegEncoder.html)
/// - WebP: [`image::codecs::webp::WebPEncoder`](https://docs.rs/image/0.25/image/codecs/webp/struct.WebPEncoder.html)
/// - AVIF: [`image::codecs::avif::AvifEncoder`](https://docs.rs/image/0.25/image/codecs/avif/struct.AvifEncoder.html)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// PNG — lossless, pixel-perfect. Default format.
    Png,
    /// JPEG — lossy compression, default quality.
    Jpeg,
    /// WebP — lossless encoding only.
    WebP,
    /// AVIF — default speed and quality settings.
    Avif,
}

impl Default for ImageFormat {
    /// Returns [`ImageFormat::Png`] — the default encoding format.
    fn default() -> Self {
        Self::Png
    }
}

impl ImageFormat {
    /// Returns the IANA media type (MIME type) for this format.
    ///
    /// # Examples
    ///
    /// ```
    /// use xshot_domain::ImageFormat;
    /// assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    /// ```
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    /// Returns the conventional file extension (without the leading dot).
    ///
    /// # Examples
    ///
    /// ```
    /// use xshot_domain::ImageFormat;
    /// assert_eq!(ImageFormat::Png.extension(), "png");
    /// ```
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WebP",
            Self::Avif => "AVIF",
        })
    }
}

/// A captured screenshot — the image payload with its dimensions and format.
///
/// `data` contains encoded image bytes in the format indicated by `format`
/// (PNG by default). The encoding is performed in the utility layer before
/// this struct is constructed.
///
/// `size` reflects the **actual** pixel dimensions of the encoded image.
/// For a full-monitor capture this matches `MonitorInfo::physical`, but
/// future region captures may produce smaller images.
#[derive(Debug, Clone)]
pub struct Screenshot {
    /// Actual pixel dimensions of the encoded image.
    pub size: Size,
    /// The encoding format of `data`.
    pub format: ImageFormat,
    /// Encoded image bytes in the format specified by `format`.
    pub data: Vec<u8>,
}

/// The result of a capture operation — pairs monitor metadata with a
/// screenshot.
///
/// Returned by [`capture_monitor`](xshot_core::capture_monitor) and
/// [`capture_all_monitors`](xshot_core::capture_all_monitors).
#[derive(Debug, Clone)]
pub struct CaptureResult {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: MonitorInfo,
    /// The captured image with its dimensions and encoded bytes.
    pub screenshot: Screenshot,
}
