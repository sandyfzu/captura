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
/// formats that captura supports.
///
/// Encoded formats (PNG, JPEG, WebP, AVIF) use default encoder settings.
/// For fine-grained control over encoding parameters (e.g. JPEG quality,
/// AVIF speed), capture as `Raw` (unencoded RGBA8 pixel data) and encode
/// externally with your preferred image library.
///
/// # Supported formats
///
/// | Variant | MIME type | Notes |
/// |---------|-----------|-------|
/// | `Raw` | `application/octet-stream` | Unencoded RGBA8 pixel data (4 bytes/pixel, row-major). Zero-copy. |
/// | `Png` | `image/png` | **Default.** Lossless, best for pixel-perfect captures. |
/// | `Jpeg` | `image/jpeg` | Lossy. Default quality (75). |
/// | `WebP` | `image/webp` | Lossless only (the `image` crate does not support lossy WebP). |
/// | `Avif` | `image/avif` | Default speed and quality settings. |
///
/// # Raw format
///
/// When `Raw` is selected, `Screenshot::data` contains the RGBA8 pixel
/// buffer — **no encoding, no compression**. The buffer layout is:
///
/// - 4 bytes per pixel: R, G, B, A (in that order).
/// - Pixels are in row-major order, top-left to bottom-right.
/// - Buffer length = `width × height × 4`.
///
/// This is the fastest capture path. Use it when you intend to process
/// pixels directly (e.g. feed into `sharp`, a canvas, WebGL textures, or
/// re-encode with custom settings).
///
/// `Raw` is **not supported** with Base64 capture functions
/// (`captureMonitorBase64`, `captureAllMonitorsBase64`). Passing `Raw` to
/// those functions returns an `INVALID_ARGUMENT` error. Raw data is not
/// self-describing and has no meaningful MIME type for data URIs.
///
/// # Sources
///
/// - Raw: [`xcap::Monitor::capture_image()`](https://docs.rs/xcap/0.9/xcap/struct.Monitor.html#method.capture_image) returns `image::RgbaImage` (`ImageBuffer<Rgba<u8>, Vec<u8>>`)
/// - PNG: [`image::codecs::png::PngEncoder`](https://docs.rs/image/0.25/image/codecs/png/struct.PngEncoder.html)
/// - JPEG: [`image::codecs::jpeg::JpegEncoder`](https://docs.rs/image/0.25/image/codecs/jpeg/struct.JpegEncoder.html)
/// - WebP: [`image::codecs::webp::WebPEncoder`](https://docs.rs/image/0.25/image/codecs/webp/struct.WebPEncoder.html)
/// - AVIF: [`image::codecs::avif::AvifEncoder`](https://docs.rs/image/0.25/image/codecs/avif/struct.AvifEncoder.html)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// Raw RGBA8 pixel data — unencoded, zero-copy. 4 bytes per pixel,
    /// row-major, top-left to bottom-right. Buffer length = width × height × 4.
    ///
    /// Not supported with Base64 capture functions.
    Raw,
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
    /// use captura_domain::ImageFormat;
    /// assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    /// ```
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Raw => "application/octet-stream",
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
    /// use captura_domain::ImageFormat;
    /// assert_eq!(ImageFormat::Png.extension(), "png");
    /// ```
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Raw => "raw",
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
            Self::Raw => "Raw",
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WebP",
            Self::Avif => "AVIF",
        })
    }
}

impl std::str::FromStr for ImageFormat {
    type Err = crate::CapturaError;

    /// Parses an image format from a string, **case-insensitively**.
    ///
    /// Accepts canonical names (`"Raw"`, `"Png"`, `"Jpeg"`, `"WebP"`,
    /// `"Avif"`) as well as any casing variant (`"raw"`, `"png"`, `"PNG"`,
    /// `"jPeG"`, etc.). The common alias `"jpg"` (any casing) is also
    /// accepted as [`ImageFormat::Jpeg`].
    ///
    /// # Errors
    ///
    /// Returns [`CapturaError::InvalidArgument`](crate::CapturaError::InvalidArgument)
    /// if the string does not match any supported format.
    ///
    /// # Examples
    ///
    /// ```
    /// use captura_domain::ImageFormat;
    ///
    /// assert_eq!("raw".parse::<ImageFormat>().unwrap(), ImageFormat::Raw);
    /// assert_eq!("png".parse::<ImageFormat>().unwrap(), ImageFormat::Png);
    /// assert_eq!("JPEG".parse::<ImageFormat>().unwrap(), ImageFormat::Jpeg);
    /// assert_eq!("jpg".parse::<ImageFormat>().unwrap(), ImageFormat::Jpeg);
    /// assert_eq!("webp".parse::<ImageFormat>().unwrap(), ImageFormat::WebP);
    /// assert_eq!("AVIF".parse::<ImageFormat>().unwrap(), ImageFormat::Avif);
    /// assert!("bmp".parse::<ImageFormat>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("raw") {
            Ok(Self::Raw)
        } else if s.eq_ignore_ascii_case("png") {
            Ok(Self::Png)
        } else if s.eq_ignore_ascii_case("jpeg") || s.eq_ignore_ascii_case("jpg") {
            Ok(Self::Jpeg)
        } else if s.eq_ignore_ascii_case("webp") {
            Ok(Self::WebP)
        } else if s.eq_ignore_ascii_case("avif") {
            Ok(Self::Avif)
        } else {
            Err(crate::CapturaError::invalid_argument(format!(
                "unsupported image format {s:?} — expected one of: raw, png, jpeg, jpg, webp, avif (case-insensitive)"
            )))
        }
    }
}

/// A captured screenshot — the image payload with its dimensions and format.
///
/// `data` contains either raw RGBA8 pixel data (`format == Raw`) or encoded
/// image bytes in the format indicated by `format` (PNG by default).
///
/// When `format` is `Raw`, `data` holds the unencoded RGBA8 buffer
/// (`width × height × 4` bytes, row-major). For all other formats, encoding
/// is performed in the utility layer before this struct is constructed.
///
/// `size` reflects the **actual** pixel dimensions of the image.
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
/// Returned by `capture_monitor` and `capture_all_monitors` in the
/// `captura_core` crate.
#[derive(Debug, Clone)]
pub struct CaptureResult {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: MonitorInfo,
    /// The captured image with its dimensions and encoded bytes.
    pub screenshot: Screenshot,
}

/// A captured screenshot with its data encoded as a Base64 string.
///
/// Identical to [`Screenshot`] except `data` is a Base64-encoded string
/// ([RFC 4648](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// standard alphabet with padding) instead of raw bytes.
///
/// This is useful when the consumer needs a string representation — for
/// example, embedding in JSON payloads, data URIs, or HTML `<img>` tags.
///
/// To construct a [data URI](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/data)
/// from this screenshot:
///
/// ```text
/// data:<mime_type>;base64,<data>
/// ```
///
/// where `<mime_type>` is obtained from [`ImageFormat::mime_type`] and
/// `<data>` is the `data` field.
#[derive(Debug, Clone)]
pub struct Base64Screenshot {
    /// Actual pixel dimensions of the encoded image.
    pub size: Size,
    /// The encoding format of the image before Base64 encoding.
    pub format: ImageFormat,
    /// Base64-encoded image bytes (RFC 4648 standard alphabet with padding).
    pub data: String,
}

/// The result of a capture-to-Base64 operation — pairs monitor metadata
/// with a Base64-encoded screenshot.
///
/// Returned by `capture_monitor_base64` and `capture_all_monitors_base64`
/// in the `captura_core` crate.
#[derive(Debug, Clone)]
pub struct Base64CaptureResult {
    /// Metadata of the monitor this screenshot was captured from.
    pub monitor: MonitorInfo,
    /// The captured image with its dimensions and Base64-encoded data.
    pub screenshot: Base64Screenshot,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ImageFormat::from_str -------------------------------------------------

    #[test]
    fn parse_png_case_insensitive() {
        for input in ["png", "Png", "PNG", "pNg", "pNG", "PnG"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::Png,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_jpeg_case_insensitive() {
        for input in ["jpeg", "Jpeg", "JPEG", "JpEg", "jPeG"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::Jpeg,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_jpg_alias_case_insensitive() {
        for input in ["jpg", "Jpg", "JPG", "jPg", "jpG"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::Jpeg,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_webp_case_insensitive() {
        for input in ["webp", "WebP", "WEBP", "Webp", "wEbP"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::WebP,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_avif_case_insensitive() {
        for input in ["avif", "Avif", "AVIF", "aViF", "AVif"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::Avif,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_raw_case_insensitive() {
        for input in ["raw", "Raw", "RAW", "rAw", "raW"] {
            assert_eq!(
                input.parse::<ImageFormat>().unwrap(),
                ImageFormat::Raw,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn parse_unknown_format_returns_invalid_argument() {
        for input in ["bmp", "gif", "tiff", "svg", "", "pn", "jpe", "web"] {
            let err = input.parse::<ImageFormat>().unwrap_err();
            assert_eq!(
                err.code(),
                crate::CapturaErrorCode::InvalidArgument,
                "failed for {input:?}"
            );
        }
    }

    #[test]
    fn from_str_error_includes_input_and_valid_formats() {
        let err = "bmp".parse::<ImageFormat>().unwrap_err();
        let msg = err.to_string();
        // The error message should contain the rejected input so the user
        // can see what went wrong.
        assert!(msg.contains("bmp"), "expected input in message: {msg}");
        // It should also hint at the valid options.
        assert!(msg.contains("raw"), "expected valid format hint: {msg}");
        assert!(msg.contains("png"), "expected valid format hint: {msg}");
        assert!(msg.contains("jpeg"), "expected valid format hint: {msg}");
        assert!(msg.contains("webp"), "expected valid format hint: {msg}");
        assert!(msg.contains("avif"), "expected valid format hint: {msg}");
    }

    /// Every variant's `Display` output should round-trip through `FromStr`.
    ///
    /// This guarantees the string representation is self-consistent:
    /// `format.to_string().parse()` always recovers the original variant.
    #[test]
    fn display_roundtrips_through_from_str() {
        for format in [
            ImageFormat::Raw,
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::WebP,
            ImageFormat::Avif,
        ] {
            let display = format.to_string();
            let parsed: ImageFormat = display
                .parse()
                .unwrap_or_else(|e| panic!("failed to parse Display output {display:?} back: {e}"));
            assert_eq!(parsed, format, "roundtrip failed for {display:?}");
        }
    }

    /// `ImageFormat` derives `Hash` — verify it works in a `HashSet`.
    #[test]
    fn image_format_usable_in_hash_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ImageFormat::Png);
        set.insert(ImageFormat::Jpeg);
        set.insert(ImageFormat::Png); // duplicate
        assert_eq!(set.len(), 2);
        assert!(set.contains(&ImageFormat::Png));
        assert!(set.contains(&ImageFormat::Jpeg));
    }

    // -- ImageFormat helpers ---------------------------------------------------

    #[test]
    fn default_is_png() {
        assert_eq!(ImageFormat::default(), ImageFormat::Png);
    }

    #[test]
    fn mime_types() {
        assert_eq!(ImageFormat::Raw.mime_type(), "application/octet-stream");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
        assert_eq!(ImageFormat::Avif.mime_type(), "image/avif");
    }

    #[test]
    fn extensions() {
        assert_eq!(ImageFormat::Raw.extension(), "raw");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
        assert_eq!(ImageFormat::Avif.extension(), "avif");
    }

    #[test]
    fn display_names() {
        assert_eq!(ImageFormat::Raw.to_string(), "Raw");
        assert_eq!(ImageFormat::Png.to_string(), "PNG");
        assert_eq!(ImageFormat::Jpeg.to_string(), "JPEG");
        assert_eq!(ImageFormat::WebP.to_string(), "WebP");
        assert_eq!(ImageFormat::Avif.to_string(), "AVIF");
    }
}
