// Integration tests for the captura native module.
//
// These tests exercise the compiled `.node` binary through the public JS API.
// Display-dependent tests are skipped automatically in headless environments
// (e.g. CI without a physical screen).
//
// Run tests:   node --test __tests__/integration.mts
// Type-check:  npx tsc --noEmit
//
// References:
//   Node.js test runner — https://nodejs.org/api/test.html
//   Node.js TypeScript  — https://nodejs.org/api/typescript.html
//   TypeScript 6.0      — https://devblogs.microsoft.com/typescript/announcing-typescript-6-0/

import { describe, it, before } from 'node:test'
import type { TestContext } from 'node:test'
import assert from 'node:assert/strict'
import {
  getMonitors,
  getMonitorById,
  captureMonitor,
  captureAllMonitors,
  captureMonitorBase64,
  captureAllMonitorsBase64,
} from '../index.js'
import type {
  Monitor,
  Bounds,
  CaptureResult,
  Base64CaptureResult,
} from '../index.js'
import {
  CapturaErrorCode,
  getCapturaErrorCode,
  isCapturaError,
} from '../errors.js'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Names of every public function the module must export. */
const EXPECTED_EXPORTS: ReadonlyMap<string, unknown> = new Map<string, unknown>([
  ['getMonitors', getMonitors],
  ['getMonitorById', getMonitorById],
  ['captureMonitor', captureMonitor],
  ['captureAllMonitors', captureAllMonitors],
  ['captureMonitorBase64', captureMonitorBase64],
  ['captureAllMonitorsBase64', captureAllMonitorsBase64],
])

/**
 * Asserts that `err` is a captura-originated error tagged with `code`.
 *
 * Uses {@link isCapturaError} (which validates the parsed `[CODE]` prefix
 * against the canonical `CapturaErrorCode` enum from the native binding)
 * rather than a plain string match — so the assertion would fail if some
 * unrelated library happened to throw an `Error` with a matching prefix.
 */
function assertCapturaError(err: unknown, code: CapturaErrorCode): true {
  assert.ok(err instanceof Error, `Expected an Error, got ${typeof err}`)
  assert.ok(
    isCapturaError(err, code),
    `Expected a captura ${code} error, got: "${err.message}" ` +
      `(parsed code: ${String(getCapturaErrorCode(err))})`,
  )
  return true
}

/**
 * Pins the wire-level `[CODE]` message prefix as a regression guard. The
 * helper-based check above is the recommended consumer API, but the prefix
 * is itself a public contract documented in README.md / AGENTS.md.
 */
function assertWirePrefix(err: unknown, code: CapturaErrorCode): true {
  assert.ok(err instanceof Error)
  assert.ok(
    err.message.startsWith(`[${code}]`),
    `Expected message to start with [${code}], got: "${err.message}"`,
  )
  return true
}

/**
 * Validates the runtime shape of a {@link Bounds} object.
 * Catches mismatches between the native binding output and the TS interface.
 */
function assertBoundsShape(b: Bounds, label: string): void {
  assert.equal(typeof b.x, 'number', `${label}.x must be a number`)
  assert.equal(typeof b.y, 'number', `${label}.y must be a number`)
  assert.equal(typeof b.width, 'number', `${label}.width must be a number`)
  assert.equal(typeof b.height, 'number', `${label}.height must be a number`)
}

/**
 * Guards a display-dependent test. Calls `t.skip()` and returns `undefined`
 * if no monitors are available; otherwise returns the monitor list.
 *
 * Callers **must** `return` immediately when the result is `undefined` so
 * the skipped test does not execute assertions against missing data.
 */
function withMonitors(
  t: TestContext,
  monitors: readonly Monitor[],
): Monitor[] | undefined {
  if (monitors.length === 0) {
    t.skip('no display available')
    return undefined
  }
  return [...monitors]
}

// ---------------------------------------------------------------------------
// 1. Exports — always pass, no display needed
// ---------------------------------------------------------------------------

describe('module exports', () => {
  for (const [name, fn] of EXPECTED_EXPORTS) {
    it(`exports ${name} as a function`, () => {
      assert.equal(typeof fn, 'function')
    })
  }
})

// ---------------------------------------------------------------------------
// 2. Error handling — always pass, no display needed
// ---------------------------------------------------------------------------

describe('error handling', () => {
  describe('invalid format string rejects with INVALID_ARGUMENT', () => {
    it('captureMonitor with invalid format', async () => {
      await assert.rejects(
        () => captureMonitor(1, 'bmp'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureAllMonitors with invalid format', async () => {
      await assert.rejects(
        () => captureAllMonitors('tiff'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureMonitorBase64 with invalid format', async () => {
      await assert.rejects(
        () => captureMonitorBase64(1, 'gif'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureAllMonitorsBase64 with invalid format', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('targa'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureAllMonitorsBase64 with Raw format', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('Raw'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureMonitorBase64 with Raw format', async () => {
      await assert.rejects(
        () => captureMonitorBase64(1, 'Raw'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('captureAllMonitorsBase64 with lowercase raw', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('raw'),
        (err: unknown) => assertCapturaError(err, CapturaErrorCode.InvalidArgument),
      )
    })

    it('also pins the [CODE] wire-format prefix', async () => {
      await assert.rejects(
        () => captureMonitor(1, 'bmp'),
        (err: unknown) => assertWirePrefix(err, CapturaErrorCode.InvalidArgument),
      )
    })
  })

  it('functions return promises', () => {
    const result: unknown = getMonitors()
    assert.ok(result instanceof Promise, 'getMonitors() must return a Promise')
    // Suppress unhandled rejection in headless environments.
    void (result as Promise<unknown>).catch(() => {})
  })
})

// ---------------------------------------------------------------------------
// 2b. Error helpers — captura/errors public API
// ---------------------------------------------------------------------------

describe('captura/errors helpers', () => {
  it('CapturaErrorCode exposes every documented category', () => {
    const expected = [
      'INITIALIZATION_ERROR',
      'MONITOR_NOT_FOUND',
      'CAPTURE_FAILED',
      'PERMISSION_DENIED',
      'PLATFORM_NOT_SUPPORTED',
      'ENCODING_ERROR',
      'INVALID_ARGUMENT',
      'INTERNAL_ERROR',
      'TIMEOUT_ERROR',
      'RESOURCE_UNAVAILABLE',
    ]
    const actual = new Set<string>()
    for (const key of Object.getOwnPropertyNames(CapturaErrorCode)) {
      const value = (CapturaErrorCode as Record<string, unknown>)[key]
      if (typeof value === 'string') actual.add(value)
    }
    for (const code of expected) {
      assert.ok(actual.has(code), `Missing CapturaErrorCode wire value: ${code}`)
    }
  })

  it('isCapturaError detects a real captura rejection', async () => {
    await assert.rejects(
      () => captureMonitor(1, 'bmp'),
      (err: unknown) => {
        assert.ok(isCapturaError(err), 'must recognize captura errors')
        assert.ok(isCapturaError(err, CapturaErrorCode.InvalidArgument))
        assert.equal(
          getCapturaErrorCode(err),
          CapturaErrorCode.InvalidArgument,
        )
        return true
      },
    )
  })

  it('isCapturaError rejects errors without a [CODE] prefix', () => {
    const plain = new Error('plain error with no prefix')
    assert.equal(isCapturaError(plain), false)
    assert.equal(getCapturaErrorCode(plain), undefined)
  })

  it('isCapturaError rejects unknown [CODE] prefixes (spoof guard)', () => {
    // Validates the core robustness requirement: a third-party error that
    // happens to use a [FOO] prefix MUST NOT be classified as captura.
    const spoof = new Error('[NOT_A_CAPTURA_CODE] spoofed message')
    assert.equal(isCapturaError(spoof), false)
    assert.equal(
      getCapturaErrorCode(spoof),
      undefined,
      'unknown prefixes must not parse as a captura code',
    )
  })

  it('isCapturaError rejects non-Error inputs', () => {
    assert.equal(isCapturaError(undefined), false)
    assert.equal(isCapturaError(null), false)
    assert.equal(isCapturaError('[INVALID_ARGUMENT] string not error'), false)
    assert.equal(isCapturaError({ message: '[INVALID_ARGUMENT] plain obj' }), false)
  })

  it('isCapturaError narrows the type with the code parameter', async (t: TestContext) => {
    // 1. "Wrong code is rejected" — exercised via captureMonitor with an
    //    invalid format string, which fails synchronously inside the interop
    //    layer (no OS monitor enumeration required), so this branch runs on
    //    every platform, including headless Linux CI.
    await assert.rejects(
      () => captureMonitor(1, 'bmp'),
      (err: unknown) => {
        assert.ok(isCapturaError(err, CapturaErrorCode.InvalidArgument))
        assert.equal(
          isCapturaError(err, CapturaErrorCode.MonitorNotFound),
          false,
          'wrong code must not match even when err is a captura error',
        )
        return true
      },
    )

    // 2. "Correct code matches" — requires a real MONITOR_NOT_FOUND, which
    //    only surfaces when xcap::Monitor::all() succeeds first. On headless
    //    Linux runners (no X11/Wayland session) that enumeration fails with
    //    [RESOURCE_UNAVAILABLE] before the ID lookup, so we skip when no
    //    display is available. macOS / Windows CI runners have a desktop
    //    session and exercise this path normally.
    let hasDisplay = false
    try {
      const ms = await getMonitors()
      hasDisplay = ms.length > 0
    } catch {
      hasDisplay = false
    }
    if (!hasDisplay) {
      t.skip('no display available — cannot exercise MonitorNotFound path')
      return
    }

    await assert.rejects(
      () => getMonitorById(0xfffffff),
      (err: unknown) => isCapturaError(err, CapturaErrorCode.MonitorNotFound),
    )
  })
})

// ---------------------------------------------------------------------------
// 3. Display-dependent tests — skipped when no display is available
// ---------------------------------------------------------------------------

describe('live capture', async () => {
  let monitors: Monitor[] = []

  before(async (): Promise<void> => {
    try {
      monitors = await getMonitors()
    } catch {
      // No display available — monitors stays empty; tests will be skipped.
    }
  })

  // -- getMonitors ----------------------------------------------------------

  it('getMonitors returns a non-empty array', (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    assert.ok(Array.isArray(ms))
    assert.ok(ms.length > 0, 'expected at least one monitor')
  })

  // -- Monitor shape --------------------------------------------------------

  it('monitor has the expected shape', (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const m = ms[0]
    assert.ok(m !== undefined, 'first monitor must exist')

    // Scalar fields
    assert.equal(typeof m.id, 'number')
    assert.equal(typeof m.name, 'string')
    assert.equal(typeof m.friendlyName, 'string')
    assert.equal(typeof m.rotation, 'number')
    assert.equal(typeof m.scaleFactor, 'number')
    assert.equal(typeof m.frequency, 'number')
    assert.equal(typeof m.isPrimary, 'boolean')
    assert.equal(typeof m.isBuiltin, 'boolean')

    // Nested Bounds objects
    assertBoundsShape(m.physical, 'physical')
    assertBoundsShape(m.logical, 'logical')
  })

  it('at least one monitor is primary', (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    assert.ok(
      ms.some((m: Monitor) => m.isPrimary),
      'no primary monitor found',
    )
  })

  // -- getMonitorById -------------------------------------------------------

  it('getMonitorById returns the correct monitor', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const expected = ms[0]
    assert.ok(expected !== undefined)

    const actual: Monitor = await getMonitorById(expected.id)
    assert.equal(actual.id, expected.id)
    assert.equal(actual.name, expected.name)
  })

  it('getMonitorById with unknown id throws MONITOR_NOT_FOUND', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    await assert.rejects(
      () => getMonitorById(0xfffffff),
      (err: unknown) => assertCapturaError(err, CapturaErrorCode.MonitorNotFound),
    )
  })

  // -- captureMonitor (Buffer) ----------------------------------------------

  it('captureMonitor returns a CaptureResult with Buffer data', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const first = ms[0]
    assert.ok(first !== undefined)

    const result: CaptureResult = await captureMonitor(first.id)

    // monitor
    assert.equal(result.monitor.id, first.id)

    // screenshot
    assert.ok(Buffer.isBuffer(result.screenshot.data), 'data must be a Buffer')
    assert.ok(result.screenshot.data.length > 0, 'data must not be empty')
    assert.equal(typeof result.screenshot.size.width, 'number')
    assert.equal(typeof result.screenshot.size.height, 'number')
    assert.ok(result.screenshot.size.width > 0)
    assert.ok(result.screenshot.size.height > 0)
    assert.equal(result.screenshot.format, 'Png')
  })

  it('captureMonitor with Jpeg format', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const first = ms[0]
    assert.ok(first !== undefined)

    const result: CaptureResult = await captureMonitor(first.id, 'Jpeg')
    assert.equal(result.screenshot.format, 'Jpeg')
    assert.ok(Buffer.isBuffer(result.screenshot.data))
    assert.ok(result.screenshot.data.length > 0)
  })

  // -- captureAllMonitors ---------------------------------------------------

  it('captureAllMonitors returns one result per monitor', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const results: CaptureResult[] = await captureAllMonitors()
    assert.ok(Array.isArray(results))
    assert.equal(results.length, ms.length)
    for (const r of results) {
      assert.ok(Buffer.isBuffer(r.screenshot.data))
      assert.ok(r.screenshot.data.length > 0)
    }
  })

  // -- captureMonitorBase64 -------------------------------------------------

  it('captureMonitorBase64 returns a Base64CaptureResult', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const first = ms[0]
    assert.ok(first !== undefined)

    const result: Base64CaptureResult = await captureMonitorBase64(first.id)

    assert.equal(result.monitor.id, first.id)

    // screenshot data must be a non-empty Base64 string
    assert.equal(typeof result.screenshot.data, 'string')
    assert.ok(result.screenshot.data.length > 0)
    assert.equal(result.screenshot.format, 'Png')

    // Verify valid Base64 by decoding
    const buf: Buffer = Buffer.from(result.screenshot.data, 'base64')
    assert.ok(buf.length > 0, 'Base64 decode must produce non-empty buffer')
  })

  // -- captureAllMonitorsBase64 ---------------------------------------------

  it('captureAllMonitorsBase64 returns one result per monitor', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const results: Base64CaptureResult[] = await captureAllMonitorsBase64()
    assert.ok(Array.isArray(results))
    assert.equal(results.length, ms.length)
    for (const r of results) {
      assert.equal(typeof r.screenshot.data, 'string')
      assert.ok(r.screenshot.data.length > 0)
    }
  })

  // -- Format support across capture functions ------------------------------

  const encodedFormats = ['Png', 'Jpeg', 'WebP', 'Avif'] as const
  const allFormats = ['Raw', ...encodedFormats] as const

  for (const fmt of allFormats) {
    it(`captureMonitor supports ${fmt} format`, async (t: TestContext) => {
      const ms = withMonitors(t, monitors)
      if (ms === undefined) return

      const first = ms[0]
      assert.ok(first !== undefined)

      const result: CaptureResult = await captureMonitor(first.id, fmt)
      assert.equal(result.screenshot.format, fmt)
      assert.ok(result.screenshot.data.length > 0)
    })
  }

  for (const fmt of encodedFormats) {
    it(`captureMonitorBase64 supports ${fmt} format`, async (t: TestContext) => {
      const ms = withMonitors(t, monitors)
      if (ms === undefined) return

      const first = ms[0]
      assert.ok(first !== undefined)

      const result: Base64CaptureResult = await captureMonitorBase64(first.id, fmt)
      assert.equal(result.screenshot.format, fmt)
      assert.ok(result.screenshot.data.length > 0)
    })
  }

  // -- Raw format specifics -------------------------------------------------

  it('captureMonitor Raw returns RGBA buffer of expected size', async (t: TestContext) => {
    const ms = withMonitors(t, monitors)
    if (ms === undefined) return

    const first = ms[0]
    assert.ok(first !== undefined)

    const result: CaptureResult = await captureMonitor(first.id, 'Raw')
    assert.equal(result.screenshot.format, 'Raw')

    const { width, height } = result.screenshot.size
    const expectedBytes = width * height * 4
    assert.equal(
      result.screenshot.data.length,
      expectedBytes,
      `Raw buffer should be ${width}×${height}×4 = ${expectedBytes} bytes, ` +
        `got ${result.screenshot.data.length}`,
    )
  })
})
