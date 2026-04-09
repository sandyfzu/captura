// Integration tests for the xshot native module.
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
 * Asserts that {@link err} is an `Error` whose `message` contains the given
 * {@link code} string. Returns `true` so it can be used as an
 * `assert.rejects` validator.
 */
function assertErrorCode(err: unknown, code: string): true {
  assert.ok(err instanceof Error, `Expected an Error, got ${typeof err}`)
  assert.ok(
    err.message.includes(code),
    `Expected "${code}" in error message, got: "${err.message}"`,
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
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureAllMonitors with invalid format', async () => {
      await assert.rejects(
        () => captureAllMonitors('tiff'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureMonitorBase64 with invalid format', async () => {
      await assert.rejects(
        () => captureMonitorBase64(1, 'gif'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureAllMonitorsBase64 with invalid format', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('targa'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureAllMonitorsBase64 with Raw format', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('Raw'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureMonitorBase64 with Raw format', async () => {
      await assert.rejects(
        () => captureMonitorBase64(1, 'Raw'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
      )
    })

    it('captureAllMonitorsBase64 with lowercase raw', async () => {
      await assert.rejects(
        () => captureAllMonitorsBase64('raw'),
        (err: unknown) => assertErrorCode(err, '[INVALID_ARGUMENT]'),
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
      (err: unknown) => assertErrorCode(err, '[MONITOR_NOT_FOUND]'),
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
