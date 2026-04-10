//! Unified error type for xshot.
//!
//! All errors across the workspace are represented by [`XshotError`]. Each
//! variant carries a structured [`XshotErrorCode`] for programmatic matching
//! on the JavaScript side.

use std::fmt;

/// Structured error code surfaced as the `code` property on JavaScript `Error`
/// objects. Consumers can `switch` on this value for programmatic handling.
///
/// # References
///
/// - AGENTS.md § Error Categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XshotErrorCode {
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

impl XshotErrorCode {
    /// Returns the string representation used as the `code` property on the
    /// JavaScript `Error` object.
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

impl fmt::Display for XshotErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The unified error type for all xshot operations.
///
/// Each variant carries a human-readable `message`. The associated
/// [`XshotErrorCode`] is derived from the variant — there is no separate
/// `code` field, which eliminates the possibility of a mismatched code.
///
/// When crossing the FFI boundary the interop layer converts this into a
/// JavaScript `Error` with a structured message:
/// `err.message === "[MONITOR_NOT_FOUND] Monitor not found: no monitor with id 42"`.
#[derive(Debug, thiserror::Error)]
pub enum XshotError {
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

impl XshotError {
    /// Returns the [`XshotErrorCode`] associated with this error.
    ///
    /// The code is derived from the enum variant — each variant maps to
    /// exactly one code, ensuring the relationship is always consistent.
    pub fn code(&self) -> XshotErrorCode {
        match self {
            Self::Initialization { .. } => XshotErrorCode::InitializationError,
            Self::MonitorNotFound { .. } => XshotErrorCode::MonitorNotFound,
            Self::CaptureFailed { .. } => XshotErrorCode::CaptureFailed,
            Self::PermissionDenied { .. } => XshotErrorCode::PermissionDenied,
            Self::PlatformNotSupported { .. } => XshotErrorCode::PlatformNotSupported,
            Self::EncodingError { .. } => XshotErrorCode::EncodingError,
            Self::InvalidArgument { .. } => XshotErrorCode::InvalidArgument,
            Self::InternalError { .. } => XshotErrorCode::InternalError,
            Self::Timeout { .. } => XshotErrorCode::TimeoutError,
            Self::ResourceUnavailable { .. } => XshotErrorCode::ResourceUnavailable,
        }
    }

    // -- Convenience constructors --

    /// Creates a [`XshotError::Initialization`] from a source error message.
    pub fn initialization(msg: impl Into<String>) -> Self {
        Self::Initialization {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::MonitorNotFound`] for the given monitor ID.
    pub fn monitor_not_found(id: u32) -> Self {
        Self::MonitorNotFound {
            message: format!("no monitor with id {id}"),
        }
    }

    /// Creates a [`XshotError::CaptureFailed`] from a source error message.
    pub fn capture_failed(msg: impl Into<String>) -> Self {
        Self::CaptureFailed {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::PermissionDenied`] from a source error message.
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::PlatformNotSupported`] from a source error message.
    pub fn platform_not_supported(msg: impl Into<String>) -> Self {
        Self::PlatformNotSupported {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::EncodingError`] from a source error message.
    pub fn encoding_error(msg: impl Into<String>) -> Self {
        Self::EncodingError {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::InvalidArgument`] from a source error message.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::InternalError`] from a source error message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalError {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::Timeout`] from a source error message.
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout {
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::ResourceUnavailable`] from a source error message.
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
            XshotError::initialization("x").code(),
            XshotErrorCode::InitializationError,
        );
        assert_eq!(
            XshotError::monitor_not_found(1).code(),
            XshotErrorCode::MonitorNotFound,
        );
        assert_eq!(
            XshotError::capture_failed("x").code(),
            XshotErrorCode::CaptureFailed,
        );
        assert_eq!(
            XshotError::permission_denied("x").code(),
            XshotErrorCode::PermissionDenied,
        );
        assert_eq!(
            XshotError::platform_not_supported("x").code(),
            XshotErrorCode::PlatformNotSupported,
        );
        assert_eq!(
            XshotError::encoding_error("x").code(),
            XshotErrorCode::EncodingError,
        );
        assert_eq!(
            XshotError::invalid_argument("x").code(),
            XshotErrorCode::InvalidArgument,
        );
        assert_eq!(
            XshotError::internal("x").code(),
            XshotErrorCode::InternalError,
        );
        assert_eq!(
            XshotError::timeout("x").code(),
            XshotErrorCode::TimeoutError,
        );
        assert_eq!(
            XshotError::resource_unavailable("x").code(),
            XshotErrorCode::ResourceUnavailable,
        );
    }

    #[test]
    fn display_is_human_readable() {
        // Verify the `#[error("...")]` template for every variant.
        // Source: thiserror v2 — https://docs.rs/thiserror/2/thiserror/#display
        assert_eq!(
            XshotError::initialization("x").to_string(),
            "Initialization failed: x",
        );
        assert_eq!(
            XshotError::monitor_not_found(42).to_string(),
            "Monitor not found: no monitor with id 42",
        );
        assert_eq!(
            XshotError::capture_failed("device busy").to_string(),
            "Capture failed: device busy",
        );
        assert_eq!(
            XshotError::permission_denied("screen recording").to_string(),
            "Permission denied: screen recording",
        );
        assert_eq!(
            XshotError::platform_not_supported("Wayland").to_string(),
            "Platform not supported: Wayland",
        );
        assert_eq!(
            XshotError::encoding_error("corrupt buffer").to_string(),
            "Encoding error: corrupt buffer",
        );
        assert_eq!(
            XshotError::invalid_argument("bad format").to_string(),
            "Invalid argument: bad format",
        );
        assert_eq!(
            XshotError::internal("unexpected").to_string(),
            "Internal error: unexpected",
        );
        assert_eq!(
            XshotError::timeout("5s elapsed").to_string(),
            "Timeout: 5s elapsed",
        );
        assert_eq!(
            XshotError::resource_unavailable("monitor disconnected").to_string(),
            "Resource unavailable: monitor disconnected",
        );
    }

    #[test]
    fn convenience_constructors_preserve_message() {
        // All constructors accept `impl Into<String>` — verify both
        // &str and String inputs are preserved verbatim.
        let msg = "hello world";
        assert!(XshotError::initialization(msg).to_string().contains(msg));
        assert!(
            XshotError::capture_failed(msg.to_owned())
                .to_string()
                .contains(msg)
        );
        assert!(XshotError::permission_denied(msg).to_string().contains(msg));
        assert!(
            XshotError::platform_not_supported(msg)
                .to_string()
                .contains(msg)
        );
        assert!(XshotError::encoding_error(msg).to_string().contains(msg));
        assert!(XshotError::invalid_argument(msg).to_string().contains(msg));
        assert!(XshotError::internal(msg).to_string().contains(msg));
        assert!(XshotError::timeout(msg).to_string().contains(msg));
        assert!(
            XshotError::resource_unavailable(msg)
                .to_string()
                .contains(msg)
        );
    }

    #[test]
    fn monitor_not_found_includes_id() {
        let err = XshotError::monitor_not_found(999);
        assert!(
            err.to_string().contains("999"),
            "expected message to contain monitor ID 999, got: {err}"
        );
    }

    /// `XshotError` must be Send + Sync because it crosses async
    /// boundaries via `tokio::task::spawn_blocking`.
    ///
    /// Source: https://doc.rust-lang.org/std/marker/trait.Send.html
    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<XshotError>();
    }

    #[test]
    fn error_code_as_str_matches_display() {
        for code in [
            XshotErrorCode::InitializationError,
            XshotErrorCode::MonitorNotFound,
            XshotErrorCode::CaptureFailed,
            XshotErrorCode::PermissionDenied,
            XshotErrorCode::PlatformNotSupported,
            XshotErrorCode::EncodingError,
            XshotErrorCode::InvalidArgument,
            XshotErrorCode::InternalError,
            XshotErrorCode::TimeoutError,
            XshotErrorCode::ResourceUnavailable,
        ] {
            assert_eq!(code.to_string(), code.as_str());
        }
    }
}
