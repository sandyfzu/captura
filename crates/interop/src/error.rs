//! Conversion from [`XshotError`] to [`napi::Error`].
//!
//! The error code is embedded in the message in the format
//! `[CODE] Human-readable description`, allowing JS consumers to match on
//! the prefix for programmatic error handling:
//!
//! ```js
//! try {
//!   await getMonitorById(999)
//! } catch (err) {
//!   console.log(err.message) // "[MONITOR_NOT_FOUND] No monitor with id 999"
//! }
//! ```

use napi::Error;
use xshot_domain::XshotError;

/// Converts a domain [`XshotError`] into a [`napi::Error`].
///
/// The error message is formatted as `[CODE] description` so that JS code
/// can programmatically identify error kinds.
pub fn to_napi(e: XshotError) -> Error {
    let code = e.code().as_str();
    let description = e.to_string();
    Error::from_reason(format!("[{code}] {description}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use xshot_domain::{XshotError, XshotErrorCode};

    /// Every [`XshotError`] variant must convert to a [`napi::Error`] whose
    /// `reason` starts with `[CODE]` and contains the human-readable message.
    ///
    /// This is the contract that JavaScript consumers rely on for
    /// programmatic error matching.
    #[test]
    fn to_napi_includes_code_prefix_and_description() {
        // (constructor, expected error code, expected substring in description)
        let cases: Vec<(XshotError, XshotErrorCode, &str)> = vec![
            (
                XshotError::initialization("init failed"),
                XshotErrorCode::InitializationError,
                "init failed",
            ),
            (
                XshotError::monitor_not_found(42),
                XshotErrorCode::MonitorNotFound,
                "no monitor with id 42",
            ),
            (
                XshotError::capture_failed("timeout"),
                XshotErrorCode::CaptureFailed,
                "timeout",
            ),
            (
                XshotError::permission_denied("screen recording"),
                XshotErrorCode::PermissionDenied,
                "screen recording",
            ),
            (
                XshotError::platform_not_supported("Wayland unstable"),
                XshotErrorCode::PlatformNotSupported,
                "Wayland unstable",
            ),
            (
                XshotError::encoding_error("PNG failed"),
                XshotErrorCode::EncodingError,
                "PNG failed",
            ),
            (
                XshotError::invalid_argument("bad format"),
                XshotErrorCode::InvalidArgument,
                "bad format",
            ),
            (
                XshotError::internal("unexpected"),
                XshotErrorCode::InternalError,
                "unexpected",
            ),
            (
                XshotError::timeout("5s elapsed"),
                XshotErrorCode::TimeoutError,
                "5s elapsed",
            ),
            (
                XshotError::resource_unavailable("monitor gone"),
                XshotErrorCode::ResourceUnavailable,
                "monitor gone",
            ),
        ];

        for (error, expected_code, expected_substr) in cases {
            let code_str = expected_code.as_str();
            let napi_err = to_napi(error);
            let reason = &napi_err.reason;

            assert!(
                reason.starts_with(&format!("[{code_str}]")),
                "expected reason to start with [{code_str}], got: {reason}"
            );
            assert!(
                reason.contains(expected_substr),
                "expected reason to contain {expected_substr:?}, got: {reason}"
            );
        }
    }

    /// The `[CODE]` prefix must appear exactly once — no double-wrapping.
    #[test]
    fn to_napi_no_double_code_prefix() {
        let err = XshotError::monitor_not_found(1);
        let napi_err = to_napi(err);
        let count = napi_err.reason.matches("[MONITOR_NOT_FOUND]").count();
        assert_eq!(count, 1, "code prefix should appear exactly once");
    }
}
