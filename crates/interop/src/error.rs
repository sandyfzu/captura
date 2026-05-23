//! Conversion from [`CapturaError`] to [`napi::Error`].
//!
//! Every domain error is converted into a JavaScript `Error` whose `message`
//! carries a structured `[CODE] description` prefix. The bracketed code
//! enables programmatic matching on the JS side:
//!
//! ```js
//! try {
//!   await getMonitorById(999)
//! } catch (err) {
//!   // err.message === "[MONITOR_NOT_FOUND] Monitor not found: no monitor with id 999"
//!   if (err.message.startsWith('[MONITOR_NOT_FOUND]')) { /* handle */ }
//! }
//! ```
//!
//! ## Why the code is in the message
//!
//! napi-rs v3 hardcodes the JS `err.code` to the `Status` enum string
//! (e.g. `"GenericFailure"`) when rejecting async promises. There is no
//! supported way to set a custom `.code` on rejected promises. Embedding
//! the domain code in the message is the standard workaround.

use captura_domain::CapturaError;
use napi::Error;

/// Converts a domain [`CapturaError`] into a [`napi::Error`].
///
/// The resulting JavaScript `Error` has a `message` of the form
/// `"[ERROR_CODE] Human-readable description"`.
pub fn to_napi(e: CapturaError) -> Error {
    let code = e.code();
    let description = e.to_string();
    Error::from_reason(format!("[{code}] {description}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use captura_domain::{CapturaError, CapturaErrorCode};

    /// Every [`CapturaError`] variant must convert to a [`napi::Error`] whose
    /// `reason` contains the `[CODE]` prefix and the human-readable message.
    #[test]
    fn to_napi_embeds_code_in_reason() {
        let cases: Vec<(CapturaError, CapturaErrorCode, &str)> = vec![
            (
                CapturaError::initialization("init failed"),
                CapturaErrorCode::InitializationError,
                "init failed",
            ),
            (
                CapturaError::monitor_not_found(42),
                CapturaErrorCode::MonitorNotFound,
                "no monitor with id 42",
            ),
            (
                CapturaError::capture_failed("timeout"),
                CapturaErrorCode::CaptureFailed,
                "timeout",
            ),
            (
                CapturaError::permission_denied("screen recording"),
                CapturaErrorCode::PermissionDenied,
                "screen recording",
            ),
            (
                CapturaError::platform_not_supported("Wayland unstable"),
                CapturaErrorCode::PlatformNotSupported,
                "Wayland unstable",
            ),
            (
                CapturaError::encoding_error("PNG failed"),
                CapturaErrorCode::EncodingError,
                "PNG failed",
            ),
            (
                CapturaError::invalid_argument("bad format"),
                CapturaErrorCode::InvalidArgument,
                "bad format",
            ),
            (
                CapturaError::internal("unexpected"),
                CapturaErrorCode::InternalError,
                "unexpected",
            ),
            (
                CapturaError::timeout("5s elapsed"),
                CapturaErrorCode::TimeoutError,
                "5s elapsed",
            ),
            (
                CapturaError::resource_unavailable("monitor gone"),
                CapturaErrorCode::ResourceUnavailable,
                "monitor gone",
            ),
        ];

        for (error, expected_code, expected_substr) in cases {
            let code_str = expected_code.as_str();
            let napi_err = to_napi(error);

            // reason must start with [CODE]
            assert!(
                napi_err.reason.starts_with(&format!("[{code_str}]")),
                "expected reason to start with [{code_str}], got: {}",
                napi_err.reason,
            );

            // reason must also contain the human-readable description
            assert!(
                napi_err.reason.contains(expected_substr),
                "expected reason to contain {expected_substr:?}, got: {}",
                napi_err.reason,
            );
        }
    }

    /// The `[CODE]` prefix must appear exactly once — no double-wrapping.
    #[test]
    fn to_napi_no_double_code_prefix() {
        let err = CapturaError::monitor_not_found(1);
        let napi_err = to_napi(err);
        let count = napi_err.reason.matches("[MONITOR_NOT_FOUND]").count();
        assert_eq!(count, 1, "code prefix should appear exactly once");
    }
}
