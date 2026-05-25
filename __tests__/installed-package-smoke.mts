// Smoke tests for the packed npm package installed into a temporary project.
// Run from the consumer project directory after installing captura tarballs:
//   node --test /path/to/__tests__/installed-package-smoke.mts

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { describe, it } from 'node:test'

type InstalledCaptura = Record<string, unknown>
type InstalledCapturaErrors = {
  CapturaErrorCode: Record<string, string>
  getCapturaErrorCode: (err: unknown) => string | undefined
  isCapturaError: (err: unknown, code?: string) => boolean
}

const smokeRequire = createRequire(join(process.cwd(), 'package.json'))

function loadInstalledPackage(): InstalledCaptura {
  return smokeRequire('captura') as InstalledCaptura
}

function loadInstalledErrorsSubpath(): InstalledCapturaErrors {
  return smokeRequire('captura/errors') as InstalledCapturaErrors
}

function installedPackageDirectory(): string {
  return dirname(smokeRequire.resolve('captura/package.json'))
}

function declaredFunctionExports(): string[] {
  const declarations = readFileSync(
    join(installedPackageDirectory(), 'index.d.ts'),
    'utf8',
  )
  const names = new Set<string>()

  for (const match of declarations.matchAll(
    /^export declare function\s+([A-Za-z_$][\w$]*)\s*\(/gm,
  )) {
    const name = match[1]
    if (name !== undefined) names.add(name)
  }

  const sorted = [...names].sort()
  assert.ok(
    sorted.length > 0,
    'installed index.d.ts must declare at least one exported function',
  )
  return sorted
}

describe('installed captura package', () => {
  const captura = loadInstalledPackage()
  const errors = loadInstalledErrorsSubpath()

  for (const name of declaredFunctionExports()) {
    it(`exports ${name} as a function`, () => {
      assert.equal(typeof captura[name], 'function')
    })
  }

  it('exposes CapturaErrorCode from the main entry', () => {
    const codes = (captura as { CapturaErrorCode?: Record<string, string> })
      .CapturaErrorCode
    assert.ok(codes && typeof codes === 'object', 'CapturaErrorCode must be exported')
    assert.equal(codes['InvalidArgument'], 'INVALID_ARGUMENT')
    assert.equal(codes['MonitorNotFound'], 'MONITOR_NOT_FOUND')
  })

  it('exposes the captura/errors subpath helpers', () => {
    assert.equal(typeof errors.isCapturaError, 'function')
    assert.equal(typeof errors.getCapturaErrorCode, 'function')
    assert.equal(typeof errors.CapturaErrorCode, 'object')
    assert.equal(errors.CapturaErrorCode.InvalidArgument, 'INVALID_ARGUMENT')
  })

  it('rejects an invalid image format with INVALID_ARGUMENT', async () => {
    const captureAllMonitors = captura.captureAllMonitors
    assert.equal(typeof captureAllMonitors, 'function')

    await assert.rejects(
      () =>
        (captureAllMonitors as (format: string) => Promise<unknown>)(
          'definitely-not-a-format',
        ),
      (err: unknown) => {
        assert.ok(err instanceof Error)
        // Pin the wire-format contract.
        assert.ok(
          err.message.startsWith('[INVALID_ARGUMENT]'),
          `Expected [INVALID_ARGUMENT] prefix, got: ${err.message}`,
        )
        // Validate through the public helper sourced from the installed package.
        assert.ok(
          errors.isCapturaError(err, errors.CapturaErrorCode.InvalidArgument),
          'installed errors.isCapturaError must classify the rejection',
        )
        assert.equal(
          errors.getCapturaErrorCode(err),
          errors.CapturaErrorCode.InvalidArgument,
        )
        return true
      },
    )
  })

  it('captura/errors helper rejects spoofed [CODE] prefixes', () => {
    const spoof = new Error('[NOT_A_REAL_CODE] not from captura')
    assert.equal(errors.isCapturaError(spoof), false)
    assert.equal(errors.getCapturaErrorCode(spoof), undefined)
  })
})