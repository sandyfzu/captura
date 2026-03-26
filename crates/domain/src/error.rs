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
/// Every variant carries:
/// - A [`XshotErrorCode`] for programmatic matching.
/// - A human-readable message produced by `thiserror`.
///
/// When crossing the FFI boundary the interop layer converts this into a
/// JavaScript `Error` with a `code` property set to
/// [`XshotErrorCode::as_str`].
#[derive(Debug, thiserror::Error)]
pub enum XshotError {
    /// Failed to initialise the capture subsystem.
    #[error("Initialization failed: {message}")]
    Initialization {
        code: XshotErrorCode,
        message: String,
    },

    /// The requested monitor was not found.
    #[error("Monitor not found: {message}")]
    MonitorNotFound {
        code: XshotErrorCode,
        message: String,
    },

    /// A capture operation failed.
    #[error("Capture failed: {message}")]
    CaptureFailed {
        code: XshotErrorCode,
        message: String,
    },

    /// The OS denied screen-capture permission.
    #[error("Permission denied: {message}")]
    PermissionDenied {
        code: XshotErrorCode,
        message: String,
    },

    /// Feature not supported on the current platform.
    #[error("Platform not supported: {message}")]
    PlatformNotSupported {
        code: XshotErrorCode,
        message: String,
    },

    /// Image encoding failed.
    #[error("Encoding error: {message}")]
    EncodingError {
        code: XshotErrorCode,
        message: String,
    },

    /// Invalid argument provided by the caller.
    #[error("Invalid argument: {message}")]
    InvalidArgument {
        code: XshotErrorCode,
        message: String,
    },

    /// Catch-all for unexpected internal failures.
    #[error("Internal error: {message}")]
    InternalError {
        code: XshotErrorCode,
        message: String,
    },

    /// Operation timed out.
    #[error("Timeout: {message}")]
    Timeout {
        code: XshotErrorCode,
        message: String,
    },

    /// An OS resource became unavailable.
    #[error("Resource unavailable: {message}")]
    ResourceUnavailable {
        code: XshotErrorCode,
        message: String,
    },
}

impl XshotError {
    /// Returns the [`XshotErrorCode`] associated with this error.
    pub fn code(&self) -> XshotErrorCode {
        match self {
            Self::Initialization { code, .. }
            | Self::MonitorNotFound { code, .. }
            | Self::CaptureFailed { code, .. }
            | Self::PermissionDenied { code, .. }
            | Self::PlatformNotSupported { code, .. }
            | Self::EncodingError { code, .. }
            | Self::InvalidArgument { code, .. }
            | Self::InternalError { code, .. }
            | Self::Timeout { code, .. }
            | Self::ResourceUnavailable { code, .. } => *code,
        }
    }

    // -- Convenience constructors --

    /// Creates a [`XshotError::MonitorNotFound`] for the given monitor ID.
    pub fn monitor_not_found(id: u32) -> Self {
        Self::MonitorNotFound {
            code: XshotErrorCode::MonitorNotFound,
            message: format!("no monitor with id {id}"),
        }
    }

    /// Creates a [`XshotError::CaptureFailed`] from a source error message.
    pub fn capture_failed(msg: impl Into<String>) -> Self {
        Self::CaptureFailed {
            code: XshotErrorCode::CaptureFailed,
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::EncodingError`] from a source error message.
    pub fn encoding_error(msg: impl Into<String>) -> Self {
        Self::EncodingError {
            code: XshotErrorCode::EncodingError,
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::InternalError`] from a source error message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalError {
            code: XshotErrorCode::InternalError,
            message: msg.into(),
        }
    }

    /// Creates a [`XshotError::ResourceUnavailable`] from a source error message.
    pub fn resource_unavailable(msg: impl Into<String>) -> Self {
        Self::ResourceUnavailable {
            code: XshotErrorCode::ResourceUnavailable,
            message: msg.into(),
        }
    }
}
