//! NAPI-rs bindings for xshot.
//!
//! This is the **only** layer that imports `napi` and `napi_derive`. It exposes
//! public functions to Node.js via `#[napi]` macros and handles conversion
//! between domain types and NAPI-compatible types.
//!
//! # Design Principles
//!
//! - Converts domain types into NAPI-compatible types for serialization.
//! - Converts Rust errors into JavaScript `Error` objects with structured codes.
//! - All exposed functions are `async` and return `Promise` to JavaScript.
//! - No `#[cfg]` attributes in this layer — all platform branching is resolved
//!   in the core or utility layers.
//! - Panics must never cross the FFI boundary.

// Uncomment when adding #[napi] functions:
// use napi_derive::napi;
