//! Unified error type for captura.
//!
//! All errors across the workspace are represented by [`CapturaError`]. Each
//! variant carries a structured [`CapturaErrorCode`] for programmatic matching
//! on the JavaScript side.

use std::fmt;

/// Structured captura domain error code.
///
/// The interop layer embeds this code in JavaScript `Error.message` as a
/// `[CODE]` prefix for programmatic handling. With napi-rs v3 async promise
/// rejections, JavaScript `err.code` is reserved for the N-API status code.
///
/// # Reserved categories
///
/// As of v1.0.0 the runtime only produces `MONITOR_NOT_FOUND`, `CAPTURE_FAILED`,
/// `ENCODING_ERROR`, `INVALID_ARGUMENT`, `INTERNAL_ERROR`, and
/// `RESOURCE_UNAVAILABLE`. The remaining variants — `INITIALIZATION_ERROR`,
/// `PERMISSION_DENIED`, `PLATFORM_NOT_SUPPORTED`, and `TIMEOUT_ERROR` — are
/// **reserved**: they are part of the stable enum for forward compatibility but
/// are not emitted by any current code path.
///
/// # References
///
/// - AGENTS.md § Error Categories
//
// TODO(1.1.0): Wire up the reserved categories where they add value:
//   - macOS `PERMISSION_DENIED`: preflight Screen Recording access via Core
//     Graphics `CGPreflightScreenCaptureAccess` before a capture and surface
//     this code when access is denied, instead of the generic `CAPTURE_FAILED`.
//   - `PLATFORM_NOT_SUPPORTED`: classify Wayland/portal "unsupported" outcomes.
//   - `TIMEOUT_ERROR`: add an optional capture timeout wrapper.
// Keep this enum, `errors.d.ts`, the README error table, and the integration
// tests in lockstep when emitting any newly-activated category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapturaErrorCode {
    /// Failure during module or runtime initialisation.
    InitializationError,
    /// The requested monitor ID does not exist.
    MonitorNotFound,
    /// A screenshot or capture operation failed.
    CaptureFailed,
    /// The OS denied screen-capture permission (common on macOS).
    PermissionDenied,
    /// The requested feature is unavailable on the current OS.
    PlatformNotSupported,
    /// Image encoding or conversion failure.
    EncodingError,
    /// An invalid parameter was passed by the caller.
    InvalidArgument,
    /// Unexpected internal failure (catch-all).
    InternalError,
    /// The operation exceeded expected time bounds.
    TimeoutError,
    /// An OS resource (monitor, window) became unavailable.
    ResourceUnavailable,
}

impl CapturaErrorCode {
    /// Returns the string representation used in the JavaScript `[CODE]`
    /// message prefix.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InitializationError => "INITIALIZATION_ERROR",
            Self::MonitorNotFound => "MONITOR_NOT_FOUND",
            Self::CaptureFailed => "CAPTURE_FAILED",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::PlatformNotSupported => "PLATFORM_NOT_SUPPORTED",
            Self::EncodingError => "ENCODING_ERROR",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::InternalError => "INTERNAL_ERROR",
            Self::TimeoutError => "TIMEOUT_ERROR",
            Self::ResourceUnavailable => "RESOURCE_UNAVAILABLE",
        }
    }
}

impl fmt::Display for CapturaErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The unified error type for all captura operations.
///
/// Each variant carries a human-readable `message`. The associated
/// [`CapturaErrorCode`] is derived from the variant — there is no separate
/// `code` field, which eliminates the possibility of a mismatched code.
///
/// When crossing the FFI boundary the interop layer converts this into a
/// JavaScript `Error` with a structured message:
/// `err.message === "[MONITOR_NOT_FOUND] Monitor not found: no monitor with id 42"`.
#[derive(Debug, thiserror::Error)]
pub enum CapturaError {
    /// Failed to initialise the capture subsystem.
    #[error("Initialization failed: {message}")]
    Initialization { message: String },

    /// The requested monitor was not found.
    #[error("Monitor not found: {message}")]
    MonitorNotFound { message: String },

    /// A capture operation failed.
    #[error("Capture failed: {message}")]
    CaptureFailed { message: String },

    /// The OS denied screen-capture permission.
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// Feature not supported on the current platform.
    #[error("Platform not supported: {message}")]
    PlatformNotSupported { message: String },

    /// Image encoding failed.
    #[error("Encoding error: {message}")]
    EncodingError { message: String },

    /// Invalid argument provided by the caller.
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    /// Catch-all for unexpected internal failures.
    #[error("Internal error: {message}")]
    InternalError { message: String },

    /// Operation timed out.
    #[error("Timeout: {message}")]
    Timeout { message: String },

    /// An OS resource became unavailable.
    #[error("Resource unavailable: {message}")]
    ResourceUnavailable { message: String },
}

impl CapturaError {
    /// Returns the [`CapturaErrorCode`] associated with this error.
    ///
    /// The code is derived from the enum variant — each variant maps to
    /// exactly one code, ensuring the relationship is always consistent.
    pub fn code(&self) -> CapturaErrorCode {
        match self {
            Self::Initialization { .. } => CapturaErrorCode::InitializationError,
            Self::MonitorNotFound { .. } => CapturaErrorCode::MonitorNotFound,
            Self::CaptureFailed { .. } => CapturaErrorCode::CaptureFailed,
            Self::PermissionDenied { .. } => CapturaErrorCode::PermissionDenied,
            Self::PlatformNotSupported { .. } => CapturaErrorCode::PlatformNotSupported,
            Self::EncodingError { .. } => CapturaErrorCode::EncodingError,
            Self::InvalidArgument { .. } => CapturaErrorCode::InvalidArgument,
            Self::InternalError { .. } => CapturaErrorCode::InternalError,
            Self::Timeout { .. } => CapturaErrorCode::TimeoutError,
            Self::ResourceUnavailable { .. } => CapturaErrorCode::ResourceUnavailable,
        }
    }

    // -- Convenience constructors --

    /// Creates a [`CapturaError::Initialization`] from a source error message.
    pub fn initialization(msg: impl Into<String>) -> Self {
        Self::Initialization {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::MonitorNotFound`] for the given monitor ID.
    pub fn monitor_not_found(id: u32) -> Self {
        Self::MonitorNotFound {
            message: format!("no monitor with id {id}"),
        }
    }

    /// Creates a [`CapturaError::CaptureFailed`] from a source error message.
    pub fn capture_failed(msg: impl Into<String>) -> Self {
        Self::CaptureFailed {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::PermissionDenied`] from a source error message.
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::PlatformNotSupported`] from a source error message.
    pub fn platform_not_supported(msg: impl Into<String>) -> Self {
        Self::PlatformNotSupported {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::EncodingError`] from a source error message.
    pub fn encoding_error(msg: impl Into<String>) -> Self {
        Self::EncodingError {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::InvalidArgument`] from a source error message.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::InternalError`] from a source error message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalError {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::Timeout`] from a source error message.
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout {
            message: msg.into(),
        }
    }

    /// Creates a [`CapturaError::ResourceUnavailable`] from a source error message.
    pub fn resource_unavailable(msg: impl Into<String>) -> Self {
        Self::ResourceUnavailable {
            message: msg.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_derived_from_variant() {
        assert_eq!(
            CapturaError::initialization("x").code(),
            CapturaErrorCode::InitializationError,
        );
        assert_eq!(
            CapturaError::monitor_not_found(1).code(),
            CapturaErrorCode::MonitorNotFound,
        );
        assert_eq!(
            CapturaError::capture_failed("x").code(),
            CapturaErrorCode::CaptureFailed,
        );
        assert_eq!(
            CapturaError::permission_denied("x").code(),
            CapturaErrorCode::PermissionDenied,
        );
        assert_eq!(
            CapturaError::platform_not_supported("x").code(),
            CapturaErrorCode::PlatformNotSupported,
        );
        assert_eq!(
            CapturaError::encoding_error("x").code(),
            CapturaErrorCode::EncodingError,
        );
        assert_eq!(
            CapturaError::invalid_argument("x").code(),
            CapturaErrorCode::InvalidArgument,
        );
        assert_eq!(
            CapturaError::internal("x").code(),
            CapturaErrorCode::InternalError,
        );
        assert_eq!(
            CapturaError::timeout("x").code(),
            CapturaErrorCode::TimeoutError,
        );
        assert_eq!(
            CapturaError::resource_unavailable("x").code(),
            CapturaErrorCode::ResourceUnavailable,
        );
    }

    #[test]
    fn display_is_human_readable() {
        // Verify the `#[error("...")]` template for every variant.
        // Source: thiserror v2 — https://docs.rs/thiserror/2/thiserror/#display
        assert_eq!(
            CapturaError::initialization("x").to_string(),
            "Initialization failed: x",
        );
        assert_eq!(
            CapturaError::monitor_not_found(42).to_string(),
            "Monitor not found: no monitor with id 42",
        );
        assert_eq!(
            CapturaError::capture_failed("device busy").to_string(),
            "Capture failed: device busy",
        );
        assert_eq!(
            CapturaError::permission_denied("screen recording").to_string(),
            "Permission denied: screen recording",
        );
        assert_eq!(
            CapturaError::platform_not_supported("Wayland").to_string(),
            "Platform not supported: Wayland",
        );
        assert_eq!(
            CapturaError::encoding_error("corrupt buffer").to_string(),
            "Encoding error: corrupt buffer",
        );
        assert_eq!(
            CapturaError::invalid_argument("bad format").to_string(),
            "Invalid argument: bad format",
        );
        assert_eq!(
            CapturaError::internal("unexpected").to_string(),
            "Internal error: unexpected",
        );
        assert_eq!(
            CapturaError::timeout("5s elapsed").to_string(),
            "Timeout: 5s elapsed",
        );
        assert_eq!(
            CapturaError::resource_unavailable("monitor disconnected").to_string(),
            "Resource unavailable: monitor disconnected",
        );
    }

    #[test]
    fn convenience_constructors_preserve_message() {
        // All constructors accept `impl Into<String>` — verify both
        // &str and String inputs are preserved verbatim.
        let msg = "hello world";
        assert!(CapturaError::initialization(msg).to_string().contains(msg));
        assert!(
            CapturaError::capture_failed(msg.to_owned())
                .to_string()
                .contains(msg)
        );
        assert!(
            CapturaError::permission_denied(msg)
                .to_string()
                .contains(msg)
        );
        assert!(
            CapturaError::platform_not_supported(msg)
                .to_string()
                .contains(msg)
        );
        assert!(CapturaError::encoding_error(msg).to_string().contains(msg));
        assert!(
            CapturaError::invalid_argument(msg)
                .to_string()
                .contains(msg)
        );
        assert!(CapturaError::internal(msg).to_string().contains(msg));
        assert!(CapturaError::timeout(msg).to_string().contains(msg));
        assert!(
            CapturaError::resource_unavailable(msg)
                .to_string()
                .contains(msg)
        );
    }

    #[test]
    fn monitor_not_found_includes_id() {
        let err = CapturaError::monitor_not_found(999);
        assert!(
            err.to_string().contains("999"),
            "expected message to contain monitor ID 999, got: {err}"
        );
    }

    /// `CapturaError` must be Send + Sync because it crosses async
    /// boundaries via `tokio::task::spawn_blocking`.
    ///
    /// Source: https://doc.rust-lang.org/std/marker/trait.Send.html
    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CapturaError>();
    }

    #[test]
    fn error_code_as_str_matches_display() {
        for code in [
            CapturaErrorCode::InitializationError,
            CapturaErrorCode::MonitorNotFound,
            CapturaErrorCode::CaptureFailed,
            CapturaErrorCode::PermissionDenied,
            CapturaErrorCode::PlatformNotSupported,
            CapturaErrorCode::EncodingError,
            CapturaErrorCode::InvalidArgument,
            CapturaErrorCode::InternalError,
            CapturaErrorCode::TimeoutError,
            CapturaErrorCode::ResourceUnavailable,
        ] {
            assert_eq!(code.to_string(), code.as_str());
        }
    }
}
