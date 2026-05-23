//! Domain models for captura.
//!
//! This crate defines the core domain types used throughout the captura workspace.
//! Types in this layer are plain Rust structs with no dependencies on NAPI-rs or
//! platform-specific capture libraries.
//!
//! # Design Principles
//!
//! - Models are plain Rust structs; they do not derive or implement NAPI traits.
//! - Contains business logic independent of any transport or binding mechanism.
//! - Error types use `thiserror` for ergonomic error definitions.

mod error;
mod monitor;

pub use error::{CapturaError, CapturaErrorCode};
pub use monitor::{
    Base64CaptureResult, Base64Screenshot, Bounds, CaptureResult, ImageFormat, MonitorInfo,
    Screenshot, Size,
};
