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
