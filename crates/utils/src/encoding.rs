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
}
