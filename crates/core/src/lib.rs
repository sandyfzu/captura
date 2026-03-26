//! Core xcap abstraction layer for xshot.
//!
//! Pure Rust logic that interfaces with the `xcap` crate. This layer handles
//! all platform-specific normalization (DPI, coordinates, monitor metadata)
//! and returns domain types.
//!
//! # Design Principles
//!
//! - No Node.js or NAPI-rs types in this layer.
//! - Wraps `xcap::Monitor`, `xcap::Window`, and related types.
//! - Platform-specific code is isolated behind `#[cfg(target_os = "...")]`
//!   in separate modules.
//! - All operations that may block are offloaded to `tokio::task::spawn_blocking`.

mod capture;

pub use capture::{capture_all_monitors, capture_monitor, get_monitor_by_id, get_monitors};
