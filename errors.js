// captura/errors — ergonomic helpers for detecting captura-originated errors.
//
// napi-rs v3 hardcodes the JavaScript `err.code` on async-promise rejections
// to a NAPI status string (e.g. `"GenericFailure"`), so captura embeds its
// domain error category as a `[CODE]` prefix on `err.message`:
//
//     err.message === "[MONITOR_NOT_FOUND] No monitor with id 999999"
//
// Checking that prefix with a plain `startsWith()` or regex is not enough on
// its own — any third-party error could happen to use a `[FOO]` prefix. The
// helpers below additionally validate the parsed code against the canonical
// set of categories exported from the native binding (`CapturaErrorCode`).
// That makes a positive match strong evidence the error originated in captura.

'use strict'

const native = require('./index.js')

// `[CODE]` must appear at the very start of the message, contain only
// uppercase letters, digits, and underscores, and start with a letter so that
// numeric noise is rejected.
const PREFIX_PATTERN = /^\[([A-Z][A-Z0-9_]*)\]/

// Frozen snapshot of every code the native binding currently understands.
// The napi-rs `string_enum` macro defines each variant as a *non-enumerable*
// own property whose value is the SCREAMING_SNAKE wire code (because the enum
// is generated with `string_enum = "UPPER_SNAKE"` on the Rust side), so we
// have to enumerate via `getOwnPropertyNames` rather than `Object.values`.
// This set always reflects the codes the native binding can actually emit.
function buildKnownCodes(enumObject) {
  if (enumObject === null || typeof enumObject !== 'object') {
    return new Set()
  }
  const codes = new Set()
  for (const key of Object.getOwnPropertyNames(enumObject)) {
    const value = enumObject[key]
    if (typeof value === 'string') {
      codes.add(value)
    }
  }
  return codes
}

const CAPTURA_ERROR_CODES = Object.freeze(buildKnownCodes(native.CapturaErrorCode))

/**
 * Returns the captura error category embedded in `err.message`, or
 * `undefined` if `err` is not a captura-originated error.
 *
 * Both conditions must hold:
 *   1. `err` is an `Error` whose `message` starts with `[SOME_CODE]`.
 *   2. `SOME_CODE` is one of the canonical codes exposed by `CapturaErrorCode`.
 *
 * @param {unknown} err
 * @returns {string | undefined}
 */
function getCapturaErrorCode(err) {
  if (!(err instanceof Error)) return undefined
  if (typeof err.message !== 'string') return undefined
  const match = PREFIX_PATTERN.exec(err.message)
  if (match === null) return undefined
  const code = match[1]
  return CAPTURA_ERROR_CODES.has(code) ? code : undefined
}

/**
 * Type-guard that returns `true` when `err` is a captura-originated error.
 * If `code` is supplied, also requires the error to carry that exact category.
 *
 * @param {unknown} err
 * @param {string} [code]
 * @returns {boolean}
 */
function isCapturaError(err, code) {
  const actual = getCapturaErrorCode(err)
  if (actual === undefined) return false
  if (code === undefined) return true
  return actual === code
}

module.exports.CapturaErrorCode = native.CapturaErrorCode
module.exports.getCapturaErrorCode = getCapturaErrorCode
module.exports.isCapturaError = isCapturaError
