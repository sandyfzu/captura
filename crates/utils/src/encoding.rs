//! Image encoding utilities.
//!
//! Converts raw RGBA pixel buffers (as returned by `xcap`) into encoded
//! image formats. Currently supports PNG; additional formats (JPEG, WebP,
//! AVIF) can be added by extending this module.
//!
//! # References
//!
//! - `image` crate v0.25 — [`image::codecs::png::PngEncoder`]
//! - `image::ImageEncoder::write_image` for raw-buffer encoding

use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use log::debug;
use std::io::Cursor;
use xshot_domain::XshotError;

/// Encodes raw RGBA pixel data into a PNG byte buffer.
///
/// # Arguments
///
/// * `rgba` — Raw pixel data in RGBA order (4 bytes per pixel).
/// * `width` — Image width in pixels.
/// * `height` — Image height in pixels.
///
/// # Errors
///
/// Returns [`XshotError::EncodingError`] if:
/// - The dimensions overflow when computing the expected buffer length.
/// - The buffer length does not match `width × height × 4`.
/// - The PNG encoder fails.
///
/// # Performance
///
/// The encoder writes into an in-memory [`Cursor<Vec<u8>>`] pre-allocated
/// to a conservative estimate of the compressed size, avoiding repeated
/// reallocations for typical screenshot dimensions.
pub fn encode_rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, XshotError> {
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

    debug!("encoding {width}×{height} RGBA → PNG ({expected_len} bytes raw)");

    // Pre-allocate ~25 % of the raw size as a rough upper bound for PNG.
    // Real-world screenshots compress well; this avoids excessive reallocs.
    let estimated = (expected_len / 4).max(1024);
    let mut buf = Cursor::new(Vec::with_capacity(estimated));

    let encoder = PngEncoder::new(&mut buf);
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| XshotError::encoding_error(format!("PNG encoding failed: {e}")))?;

    let result = buf.into_inner();
    debug!(
        "PNG encoding complete: {width}×{height} → {} bytes",
        result.len()
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_1x1_pixel() {
        // Single red pixel: R=255, G=0, B=0, A=255
        let rgba = [255u8, 0, 0, 255];
        let png = encode_rgba_to_png(&rgba, 1, 1).expect("encoding should succeed");

        // Verify PNG magic bytes (RFC 2083 §3.1)
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(!png.is_empty());
    }

    #[test]
    fn encode_dimension_mismatch_fails() {
        // 2×2 requires 16 bytes of RGBA data; we only provide 4
        let rgba = [0u8; 4];
        let result = encode_rgba_to_png(&rgba, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn encode_overflow_dimensions_fails() {
        // u32::MAX × u32::MAX × 4 overflows usize on any platform.
        let result = encode_rgba_to_png(&[], u32::MAX, u32::MAX);
        assert!(result.is_err());
    }
}
