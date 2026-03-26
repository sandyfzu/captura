//! Utility helpers for xshot.
//!
//! Shared utilities including error conversion, image encoding, buffer
//! manipulation, and cross-platform normalization logic.
//!
//! # Design Principles
//!
//! - Used by multiple layers; must not depend on NAPI types.
//! - Image encoding (PNG, JPEG, WebP, AVIF, etc.) is handled entirely here.
//! - The interop layer passes encoding format options through; it does not
//!   contain encoding logic.

mod encoding;

pub use encoding::encode_rgba;
