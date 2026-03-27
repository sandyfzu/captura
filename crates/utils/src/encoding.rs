//! Image encoding utilities.
//!
//! Converts raw RGBA pixel buffers into encoded image formats. Supports
//! PNG, JPEG, WebP, and AVIF. Each format uses its encoder's default
//! settings — no fine-grained configuration is exposed.
//!
//! # Supported formats
//!
//! | Format | Codec | Notes |
//! |--------|-------|-------|
//! | PNG | [`image::codecs::png::PngEncoder`] | Lossless, pixel-perfect. Default format. |
//! | JPEG | [`image::codecs::jpeg::JpegEncoder`] | Lossy, default quality (75). |
//! | WebP | [`image::codecs::webp::WebPEncoder`] | **Lossless only** in the `image` crate. |
//! | AVIF | [`image::codecs::avif::AvifEncoder`] | Default speed and quality settings. |
//!
//! # References
//!
//! - [`image::ImageEncoder::write_image`] — common trait for all encoders

use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use image::ImageEncoder;
use image::codecs::avif::AvifEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use log::debug;
use std::io::Cursor;
use xshot_domain::{ImageFormat, XshotError};

/// Encodes raw RGBA pixel data into the specified image format.
///
/// # Arguments
///
/// * `rgba` — Raw pixel data in RGBA order (4 bytes per pixel).
/// * `width` — Image width in pixels.
/// * `height` — Image height in pixels.
/// * `format` — Target encoding format.
///
/// # Errors
///
/// Returns [`XshotError::InvalidArgument`] if:
/// - Either `width` or `height` is zero.
///
/// Returns [`XshotError::EncodingError`] if:
/// - The dimensions overflow when computing the expected buffer length.
/// - The buffer length does not match `width × height × 4`.
/// - The encoder fails for the chosen format.
///
/// # Performance
///
/// The encoder writes into an in-memory [`Cursor<Vec<u8>>`] pre-allocated
/// to a format-appropriate estimate of the compressed size, avoiding
/// repeated reallocations for typical screenshot dimensions.
pub fn encode_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: ImageFormat,
) -> Result<Vec<u8>, XshotError> {
    if width == 0 || height == 0 {
        return Err(XshotError::invalid_argument(format!(
            "image dimensions must be non-zero, got {width}×{height}"
        )));
    }

    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| {
            XshotError::encoding_error(format!(
                "dimensions overflow: {width}×{height} exceeds addressable memory"
            ))
        })?;

    if rgba.len() != expected_len {
        return Err(XshotError::encoding_error(format!(
            "buffer length mismatch: expected {expected_len} bytes for \
             {width}×{height} RGBA image, got {}",
            rgba.len()
        )));
    }

    debug!("encoding {width}×{height} RGBA → {format} ({expected_len} bytes raw)");

    let estimated = estimate_output_size(expected_len, format);
    let mut buf = Cursor::new(Vec::with_capacity(estimated));

    encode_into(&mut buf, rgba, width, height, format)?;

    let result = buf.into_inner();
    debug!(
        "{format} encoding complete: {width}×{height} → {} bytes",
        result.len()
    );
    Ok(result)
}

/// Encodes raw RGBA pixel data into the specified image format and then
/// Base64-encodes the result.
///
/// This is a convenience function that composes [`encode_rgba`] with
/// [RFC 4648 standard Base64](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
/// encoding (alphabet `A–Z`, `a–z`, `0–9`, `+`, `/` with `=` padding).
///
/// # Arguments
///
/// * `rgba` — Raw pixel data in RGBA order (4 bytes per pixel).
/// * `width` — Image width in pixels.
/// * `height` — Image height in pixels.
/// * `format` — Target image encoding format (applied before Base64).
///
/// # Errors
///
/// Returns the same errors as [`encode_rgba`] — the Base64 step itself is
/// infallible for bounded input.
///
/// # Implementation
///
/// Uses the [`base64`](https://docs.rs/base64/0.22/base64/) crate’s
/// `BASE64_STANDARD` engine (RFC 4648 § 4).
pub fn encode_rgba_base64(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: ImageFormat,
) -> Result<String, XshotError> {
    let encoded = encode_rgba(rgba, width, height, format)?;

    let base64_str = BASE64_STANDARD.encode(&encoded);

    debug!(
        "{format} base64 encoding complete: {width}×{height} → {} chars",
        base64_str.len()
    );

    Ok(base64_str)
}

/// Returns a rough estimate of the encoded output size to pre-allocate the
/// buffer. The goal is to reduce reallocations, not to be precise.
fn estimate_output_size(raw_len: usize, format: ImageFormat) -> usize {
    let ratio = match format {
        // PNG: lossless, ~25 % of raw for typical screenshots.
        ImageFormat::Png => 4,
        // JPEG: lossy, ~10 % of raw at default quality.
        ImageFormat::Jpeg => 10,
        // WebP (lossless): ~20 % of raw — similar to PNG.
        ImageFormat::WebP => 5,
        // AVIF: very efficient, ~10 % of raw.
        ImageFormat::Avif => 10,
    };
    (raw_len / ratio).max(1024)
}

/// Dispatches to the format-specific encoder and writes into `writer`.
fn encode_into<W: std::io::Write>(
    writer: &mut W,
    rgba: &[u8],
    width: u32,
    height: u32,
    format: ImageFormat,
) -> Result<(), XshotError> {
    match format {
        ImageFormat::Png => {
            PngEncoder::new(writer)
                .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| XshotError::encoding_error(format!("PNG encoding failed: {e}")))?;
        }
        ImageFormat::Jpeg => {
            // JPEG does not support alpha. Convert RGBA → RGB.
            let rgb = rgba_to_rgb(rgba);
            JpegEncoder::new(writer)
                .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)
                .map_err(|e| XshotError::encoding_error(format!("JPEG encoding failed: {e}")))?;
        }
        ImageFormat::WebP => {
            WebPEncoder::new_lossless(writer)
                .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| XshotError::encoding_error(format!("WebP encoding failed: {e}")))?;
        }
        ImageFormat::Avif => {
            AvifEncoder::new(writer)
                .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| XshotError::encoding_error(format!("AVIF encoding failed: {e}")))?;
        }
    }

    Ok(())
}

/// Strips the alpha channel from RGBA data, producing RGB.
fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let pixel_count = rgba.len() / 4;
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for pixel in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&pixel[..3]);
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2×2 red pixel RGBA data (16 bytes).
    fn red_2x2() -> [u8; 16] {
        let mut rgba = [0u8; 16];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[255, 0, 0, 255]);
        }
        rgba
    }

    // -- PNG ----------------------------------------------------------------

    #[test]
    fn encode_png_1x1() {
        let rgba = [255u8, 0, 0, 255];
        let png = encode_rgba(&rgba, 1, 1, ImageFormat::Png).expect("PNG encoding should succeed");
        // PNG magic bytes (RFC 2083 §3.1)
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    // -- JPEG ---------------------------------------------------------------

    #[test]
    fn encode_jpeg_2x2() {
        let rgba = red_2x2();
        let jpeg =
            encode_rgba(&rgba, 2, 2, ImageFormat::Jpeg).expect("JPEG encoding should succeed");
        // JPEG magic bytes: SOI marker = 0xFF 0xD8
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    }

    // -- WebP ---------------------------------------------------------------

    #[test]
    fn encode_webp_2x2() {
        let rgba = red_2x2();
        let webp =
            encode_rgba(&rgba, 2, 2, ImageFormat::WebP).expect("WebP encoding should succeed");
        // WebP files start with "RIFF"
        assert!(webp.starts_with(b"RIFF"));
    }

    // -- AVIF ---------------------------------------------------------------

    #[test]
    fn encode_avif_2x2() {
        let rgba = red_2x2();
        let avif =
            encode_rgba(&rgba, 2, 2, ImageFormat::Avif).expect("AVIF encoding should succeed");
        assert!(!avif.is_empty());
    }

    // -- Error cases --------------------------------------------------------

    #[test]
    fn encode_dimension_mismatch_fails() {
        // 2×2 requires 16 bytes; we only provide 4
        let rgba = [0u8; 4];
        let result = encode_rgba(&rgba, 2, 2, ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    fn encode_zero_width_fails() {
        let result = encode_rgba(&[], 0, 1, ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    fn encode_zero_height_fails() {
        let result = encode_rgba(&[], 1, 0, ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    fn encode_overflow_dimensions_fails() {
        // u32::MAX × u32::MAX × 4 overflows usize
        let result = encode_rgba(&[], u32::MAX, u32::MAX, ImageFormat::Png);
        assert!(result.is_err());
    }

    // -- Base64 -------------------------------------------------------------

    #[test]
    fn encode_rgba_base64_1x1_png() {
        let rgba = [255u8, 0, 0, 255];
        let b64 = encode_rgba_base64(&rgba, 1, 1, ImageFormat::Png)
            .expect("base64 PNG encoding should succeed");
        // Decode and verify PNG magic bytes survive the round-trip.
        let decoded = BASE64_STANDARD
            .decode(&b64)
            .expect("output should be valid base64");
        assert!(decoded.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn encode_rgba_base64_zero_width_fails() {
        let result = encode_rgba_base64(&[], 0, 1, ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    fn encode_rgba_base64_zero_height_fails() {
        let result = encode_rgba_base64(&[], 1, 0, ImageFormat::Png);
        assert!(result.is_err());
    }

    // -- Base64 for non-PNG formats -----------------------------------------

    #[test]
    fn encode_jpeg_base64_roundtrip() {
        let rgba = red_2x2();
        let b64 = encode_rgba_base64(&rgba, 2, 2, ImageFormat::Jpeg)
            .expect("JPEG base64 encoding should succeed");
        let decoded = BASE64_STANDARD
            .decode(&b64)
            .expect("output should be valid base64");
        // JPEG SOI marker: 0xFF 0xD8
        assert!(decoded.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn encode_webp_base64_roundtrip() {
        let rgba = red_2x2();
        let b64 = encode_rgba_base64(&rgba, 2, 2, ImageFormat::WebP)
            .expect("WebP base64 encoding should succeed");
        let decoded = BASE64_STANDARD
            .decode(&b64)
            .expect("output should be valid base64");
        assert!(decoded.starts_with(b"RIFF"));
    }

    #[test]
    fn encode_avif_base64_roundtrip() {
        let rgba = red_2x2();
        let b64 = encode_rgba_base64(&rgba, 2, 2, ImageFormat::Avif)
            .expect("AVIF base64 encoding should succeed");
        let decoded = BASE64_STANDARD
            .decode(&b64)
            .expect("output should be valid base64");
        assert!(!decoded.is_empty());
    }

    // -- Additional error cases ---------------------------------------------

    #[test]
    fn encode_both_dims_zero_fails() {
        let result = encode_rgba(&[], 0, 0, ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    fn encode_buffer_too_large_fails() {
        // 1×1 RGBA requires exactly 4 bytes; provide 8.
        let result = encode_rgba(&[0u8; 8], 1, 1, ImageFormat::Png);
        assert!(result.is_err());
    }

    // -- Error code classification ------------------------------------------

    /// Zero dimensions → `InvalidArgument` (caller's fault).
    #[test]
    fn error_code_zero_dims_is_invalid_argument() {
        let err = encode_rgba(&[], 0, 1, ImageFormat::Png).unwrap_err();
        assert_eq!(err.code(), XshotError::invalid_argument("").code());
    }

    /// Buffer/dimension mismatch → `EncodingError` (data integrity).
    #[test]
    fn error_code_buffer_mismatch_is_encoding_error() {
        let err = encode_rgba(&[0u8; 4], 2, 2, ImageFormat::Png).unwrap_err();
        assert_eq!(err.code(), XshotError::encoding_error("").code());
    }

    /// Dimension overflow → `EncodingError`.
    #[test]
    fn error_code_overflow_is_encoding_error() {
        let err = encode_rgba(&[], u32::MAX, u32::MAX, ImageFormat::Png).unwrap_err();
        assert_eq!(err.code(), XshotError::encoding_error("").code());
    }

    // -- Larger image -------------------------------------------------------

    /// Encode a 100×100 solid red image to verify encoding works
    /// beyond trivial 1×1 / 2×2 sizes and the pre-allocation heuristic
    /// in `estimate_output_size` doesn't cause issues.
    #[test]
    fn encode_png_100x100() {
        let rgba: Vec<u8> = [255, 0, 0, 255].repeat(100 * 100);
        let png = encode_rgba(&rgba, 100, 100, ImageFormat::Png)
            .expect("100×100 PNG encoding should succeed");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    // -- JPEG alpha stripping -----------------------------------------------

    /// JPEG does not support alpha. Verify that a semi-transparent image
    /// encodes successfully — this exercises the internal `rgba_to_rgb` path.
    #[test]
    fn jpeg_encodes_semi_transparent_pixels() {
        // 2×2 image: half transparent red, half opaque blue.
        #[rustfmt::skip]
        let rgba: [u8; 16] = [
            255, 0, 0, 128,   // semi-transparent red
            255, 0, 0, 128,
            0, 0, 255, 255,   // opaque blue
            0, 0, 255, 255,
        ];
        let jpeg = encode_rgba(&rgba, 2, 2, ImageFormat::Jpeg)
            .expect("JPEG should encode despite alpha channel");
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    }
}
