/**
 * Ergonomic helpers for detecting captura-originated errors.
 *
 * captura embeds its domain error category as a `[CODE]` prefix on
 * `err.message` (e.g. `"[MONITOR_NOT_FOUND] No monitor with id 999999"`).
 * These helpers parse that prefix and validate it against the canonical
 * `CapturaErrorCode` enum exported by the native binding, so a positive
 * match is strong evidence the error actually originated in captura.
 *
 * @example
 *   import { isCapturaError, CapturaErrorCode } from 'captura/errors'
 *
 *   try {
 *     await getMonitorById(999999)
 *   } catch (err) {
 *     if (isCapturaError(err, CapturaErrorCode.MonitorNotFound)) {
 *       // …
 *     }
 *   }
 */

/**
 * Canonical captura error categories.
 *
 * The runtime value is re-exported from the native binding's
 * `CapturaErrorCode` enum, so `CapturaErrorCode.MonitorNotFound` and
 * `'MONITOR_NOT_FOUND'` are interchangeable on the wire.
 *
 * This module declares its own local type / value pair (rather than
 * re-exporting the ambient `const enum` from `index.d.ts`) so it stays
 * compatible with `verbatimModuleSyntax` and `isolatedModules`.
 *
 * @remarks
 * `INITIALIZATION_ERROR`, `PERMISSION_DENIED`, `PLATFORM_NOT_SUPPORTED`, and
 * `TIMEOUT_ERROR` are **reserved**: they are part of the stable enum for
 * forward compatibility but are not emitted by any current code path.
 */
export type CapturaErrorCode =
  | 'INITIALIZATION_ERROR'
  | 'MONITOR_NOT_FOUND'
  | 'CAPTURE_FAILED'
  | 'PERMISSION_DENIED'
  | 'PLATFORM_NOT_SUPPORTED'
  | 'ENCODING_ERROR'
  | 'INVALID_ARGUMENT'
  | 'INTERNAL_ERROR'
  | 'TIMEOUT_ERROR'
  | 'RESOURCE_UNAVAILABLE'

export const CapturaErrorCode: {
  readonly InitializationError: 'INITIALIZATION_ERROR'
  readonly MonitorNotFound: 'MONITOR_NOT_FOUND'
  readonly CaptureFailed: 'CAPTURE_FAILED'
  readonly PermissionDenied: 'PERMISSION_DENIED'
  readonly PlatformNotSupported: 'PLATFORM_NOT_SUPPORTED'
  readonly EncodingError: 'ENCODING_ERROR'
  readonly InvalidArgument: 'INVALID_ARGUMENT'
  readonly InternalError: 'INTERNAL_ERROR'
  readonly TimeoutError: 'TIMEOUT_ERROR'
  readonly ResourceUnavailable: 'RESOURCE_UNAVAILABLE'
}

/**
 * Returns the captura error category embedded in `err.message`, or
 * `undefined` if `err` is not a captura-originated error.
 *
 * Both conditions must hold:
 *
 *   1. `err` is an `Error` whose `message` starts with `[SOME_CODE]`.
 *   2. `SOME_CODE` is one of the canonical codes exposed by `CapturaErrorCode`.
 */
export declare function getCapturaErrorCode(err: unknown): CapturaErrorCode | undefined

/**
 * Type-guard that returns `true` when `err` is a captura-originated error.
 * If `code` is supplied, also requires the error to carry that exact category.
 */
export declare function isCapturaError(err: unknown, code?: CapturaErrorCode): err is Error
