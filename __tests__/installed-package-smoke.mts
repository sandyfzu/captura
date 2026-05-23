// Smoke tests for the packed npm package installed into a temporary project.
// Run from the consumer project directory after installing captura tarballs:
//   node --test /path/to/__tests__/installed-package-smoke.mts

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { describe, it } from 'node:test'

type InstalledCaptura = Record<string, unknown>

const smokeRequire = createRequire(join(process.cwd(), 'package.json'))

function loadInstalledPackage(): InstalledCaptura {
  return smokeRequire('captura') as InstalledCaptura
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

function assertInvalidArgument(error: unknown): true {
  assert.ok(error instanceof Error, `Expected Error, got ${typeof error}`)
  assert.ok(
    error.message.startsWith('[INVALID_ARGUMENT]'),
    `Expected [INVALID_ARGUMENT] prefix, got: ${error.message}`,
  )
  return true
}

describe('installed captura package', () => {
  const captura = loadInstalledPackage()

  for (const name of declaredFunctionExports()) {
    it(`exports ${name} as a function`, () => {
      assert.equal(typeof captura[name], 'function')
    })
  }

  it('rejects an invalid image format with INVALID_ARGUMENT', async () => {
    const captureAllMonitors = captura.captureAllMonitors
    assert.equal(typeof captureAllMonitors, 'function')

    await assert.rejects(
      () =>
        (captureAllMonitors as (format: string) => Promise<unknown>)(
          'definitely-not-a-format',
        ),
      assertInvalidArgument,
    )
  })
})