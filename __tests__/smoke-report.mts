/**
 * Smoke report for captura.
 *
 * This script intentionally records observations instead of asserting pass/fail
 * conditions. It exercises every public JavaScript API exported by captura,
 * captures every supported image format variant, and writes a timestamped report
 * that can be inspected in a browser for correctness.
 *
 * Run with:
 *
 *   npm run test:smoke-report
 *
 * The npm script performs a production NAPI build before this file runs so the
 * report reflects release-like behavior. Generated screenshots can contain
 * private desktop contents, so output is written under the ignored
 * captura-smoke-reports/ directory.
 */

import { Buffer } from 'node:buffer'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { availableParallelism, cpus, freemem, hostname, release, totalmem, type as osType, uptime as osUptime } from 'node:os'
import { dirname, relative, resolve, sep } from 'node:path'
import { performance } from 'node:perf_hooks'
import process from 'node:process'
import { fileURLToPath, pathToFileURL } from 'node:url'

import {
  captureAllMonitors,
  captureAllMonitorsBase64,
  captureMonitor,
  captureMonitorBase64,
  getMonitorById,
  getMonitors,
} from '../index.js'
import type { Base64CaptureResult, CaptureResult, Monitor, Size } from '../index.js'

type ImageFormatName = 'Raw' | 'Png' | 'Jpeg' | 'WebP' | 'Avif'
type EncodedImageFormatName = 'Png' | 'Jpeg' | 'WebP' | 'Avif'
type PublicFunctionName =
  | 'getMonitors'
  | 'getMonitorById'
  | 'captureMonitor'
  | 'captureAllMonitors'
  | 'captureMonitorBase64'
  | 'captureAllMonitorsBase64'

type CallStatus = 'success' | 'error'
type ArtifactKind = 'html-report' | 'json' | 'image' | 'raw-rgba' | 'base64-text'

interface PackageMetadata {
  name: string | null
  version: string | null
  nodeEngine: string | null
  readError: SerializedError | null
}

interface EnvironmentSnapshot {
  nodeVersion: string
  nodeVersions: NodeJS.ProcessVersions
  typescriptRuntimeSupport: string | boolean | null
  platform: NodeJS.Platform
  arch: string
  pid: number
  cwd: string
  hostName: string
  osType: string
  osRelease: string
  osUptimeSeconds: number
  cpuCount: number
  availableParallelism: number
  totalMemoryBytes: number
  freeMemoryBytes: number
  processUptimeSeconds: number
  memoryUsage: NodeJS.MemoryUsage
}

interface ReportContext {
  projectRoot: string
  reportsRoot: string
  runDir: string
  reportStartedAt: Date
  sequence: number
  artifacts: ArtifactRecord[]
  calls: CallRecord[]
}

interface ArtifactRecord {
  kind: ArtifactKind
  relativePath: string
  bytes: number
  sha256: string
  description: string
  mimeType: string | null
  publicFunction: PublicFunctionName | null
  monitorId: number | null
  format: string | null
  callId: string | null
}

interface SerializedError {
  name: string
  message: string
  code: string | number | null
  stack: string | null
}

interface CallRecord {
  id: string
  label: string
  publicFunction: PublicFunctionName
  variant: string
  monitorId: number | null
  monitorName: string | null
  expectedToFail: boolean
  status: CallStatus
  outcome: string
  startedAtIso: string
  endedAtIso: string
  durationMs: number
  result: CallResultSummary | null
  error: SerializedError | null
  artifactRelativePaths: string[]
}

interface CallResultSummary {
  kind: string
  monitorCount?: number
  monitorTopology?: string
  monitors?: Monitor[]
  monitor?: Monitor
  capture?: StoredCapture
  captures?: StoredCapture[]
  note?: string
}

interface StoredCapture {
  source: PublicFunctionName
  callId: string
  monitor: Monitor
  screenshot: {
    dataKind: 'Buffer' | 'Base64'
    format: string
    mimeType: string
    size: Size
    pixelCount: number | null
    byteLength: number
    base64Length: number | null
    sha256: string
    expectedRawByteLength: number | null
    rawByteLengthMatchesExpected: boolean | null
    matchesMonitorPhysicalSize: boolean | null
    imageRelativePath: string | null
    rawRelativePath: string | null
    base64RelativePath: string | null
  }
}

interface CallRunOptions<T> {
  label: string
  publicFunction: PublicFunctionName
  variant: string
  monitor: Monitor | null
  expectedToFail?: boolean
  run: () => Promise<T>
  summarize: (value: T, context: ReportContext, call: CallRecord) => Promise<CallResultSummary>
}

interface TimingAggregate {
  key: string
  callCount: number
  successCount: number
  errorCount: number
  expectedErrorCount: number
  totalDurationMs: number
  averageDurationMs: number
  minDurationMs: number
  maxDurationMs: number
  totalArtifactBytes: number
}

interface FinalReport {
  generatedAtIso: string
  completedAtIso: string
  durationMs: number
  reportRootName: string
  reportDirectory: string
  reportUrl: string
  package: PackageMetadata
  environment: EnvironmentSnapshot
  monitorTopology: string
  monitorCount: number
  monitors: Monitor[]
  functionsCovered: PublicFunctionName[]
  formatsCovered: ImageFormatName[]
  encodedFormatsCovered: EncodedImageFormatName[]
  totals: {
    calls: number
    successes: number
    errors: number
    expectedErrors: number
    artifacts: number
    artifactBytes: number
    captures: number
  }
  calls: CallRecord[]
  artifacts: ArtifactRecord[]
  aggregates: {
    byFunction: TimingAggregate[]
    byFunctionAndVariant: TimingAggregate[]
    byMonitor: TimingAggregate[]
  }
}

const REPORTS_ROOT_NAME = 'captura-smoke-reports'
const ARTIFACTS_DIR_NAME = 'artifacts'
const MISSING_MONITOR_ID = 0x7fffffff
const LOG_LABEL_WIDTH = 18

const PUBLIC_FUNCTIONS: readonly PublicFunctionName[] = [
  'getMonitors',
  'getMonitorById',
  'captureMonitor',
  'captureAllMonitors',
  'captureMonitorBase64',
  'captureAllMonitorsBase64',
]

const ALL_FORMATS: readonly ImageFormatName[] = ['Raw', 'Png', 'Jpeg', 'WebP', 'Avif']
const ENCODED_FORMATS: readonly EncodedImageFormatName[] = ['Png', 'Jpeg', 'WebP', 'Avif']

/**
 * Creates the report context and all fixed subdirectories for this run.
 */
async function createReportContext(): Promise<ReportContext> {
  logSection('Preparing Report Workspace')
  const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  const reportsRoot = resolve(projectRoot, REPORTS_ROOT_NAME)
  const reportStartedAt = new Date()
  logProgress('started', reportStartedAt.toISOString())
  logProgress('project root', projectRoot)
  logProgress('reports root', reportsRoot)

  const runDir = await createUniqueRunDirectory(reportsRoot, reportStartedAt)
  logProgress('run directory', runDir)

  logProgress('artifact dirs', 'base64, calls, data, images, raw')
  await Promise.all([
    mkdir(resolve(runDir, ARTIFACTS_DIR_NAME, 'base64'), { recursive: true }),
    mkdir(resolve(runDir, ARTIFACTS_DIR_NAME, 'calls'), { recursive: true }),
    mkdir(resolve(runDir, ARTIFACTS_DIR_NAME, 'data'), { recursive: true }),
    mkdir(resolve(runDir, ARTIFACTS_DIR_NAME, 'images'), { recursive: true }),
    mkdir(resolve(runDir, ARTIFACTS_DIR_NAME, 'raw'), { recursive: true }),
  ])

  return {
    projectRoot,
    reportsRoot,
    runDir,
    reportStartedAt,
    sequence: 0,
    artifacts: [],
    calls: [],
  }
}

/**
 * Creates a timestamped run directory. If two runs start in the same
 * millisecond, a numeric suffix is added instead of overwriting a report.
 */
async function createUniqueRunDirectory(reportsRoot: string, date: Date): Promise<string> {
  await mkdir(reportsRoot, { recursive: true })
  const baseName = formatDateForDirectory(date)

  for (let attempt = 0; attempt < 100; attempt += 1) {
    const directoryName = attempt === 0 ? baseName : `${baseName}-${attempt + 1}`
    const candidate = resolve(reportsRoot, directoryName)
    try {
      await mkdir(candidate, { recursive: false })
      return candidate
    } catch (error) {
      if (getErrorCode(error) !== 'EEXIST') {
        throw error
      }
    }
  }

  throw new Error(`Unable to create a unique report directory under ${reportsRoot}`)
}

/**
 * Formats a date with local time components while staying portable on Windows,
 * macOS, and Linux file systems.
 */
function formatDateForDirectory(date: Date): string {
  const timezoneOffsetMinutes = -date.getTimezoneOffset()
  const timezoneSign = timezoneOffsetMinutes >= 0 ? 'plus' : 'minus'
  const absoluteOffset = Math.abs(timezoneOffsetMinutes)
  const timezoneHours = Math.trunc(absoluteOffset / 60)
  const timezoneMinutes = absoluteOffset % 60

  return [
    `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`,
    `T${pad2(date.getHours())}-${pad2(date.getMinutes())}-${pad2(date.getSeconds())}`,
    `.${String(date.getMilliseconds()).padStart(3, '0')}`,
    `${timezoneSign}${pad2(timezoneHours)}-${pad2(timezoneMinutes)}`,
  ].join('')
}

/**
 * Records one API call, captures elapsed time, stores its summarized output,
 * and continues gracefully when the API throws.
 */
async function recordCall<T>(context: ReportContext, options: CallRunOptions<T>): Promise<CallRecord> {
  context.sequence += 1
  const callId = `${String(context.sequence).padStart(3, '0')}-${slug(options.label)}`
  const startedAt = new Date()
  const startedMs = performance.now()
  const artifactStartIndex = context.artifacts.length

  const call: CallRecord = {
    id: callId,
    label: options.label,
    publicFunction: options.publicFunction,
    variant: options.variant,
    monitorId: options.monitor?.id ?? null,
    monitorName: options.monitor === null ? null : displayMonitorName(options.monitor),
    expectedToFail: options.expectedToFail ?? false,
    status: 'success',
    outcome: 'recorded',
    startedAtIso: startedAt.toISOString(),
    endedAtIso: startedAt.toISOString(),
    durationMs: 0,
    result: null,
    error: null,
    artifactRelativePaths: [],
  }

  logCallStart(call)

  try {
    const value = await options.run()
    call.result = await options.summarize(value, context, call)
    call.status = 'success'
    call.outcome = call.expectedToFail ? 'unexpected success' : 'recorded'
  } catch (error) {
    call.status = 'error'
    call.error = serializeError(error)
    call.outcome = call.expectedToFail ? 'expected error recorded' : 'error recorded'
  } finally {
    call.endedAtIso = new Date().toISOString()
    call.durationMs = performance.now() - startedMs
    const callJsonRelativePath = [ARTIFACTS_DIR_NAME, 'calls', `${call.id}.json`].join('/')
    call.artifactRelativePaths = context.artifacts
      .slice(artifactStartIndex)
      .map((artifact) => artifact.relativePath)
      .concat(callJsonRelativePath)

    context.calls.push(call)
    await writeJsonArtifact(context, {
      relativePathParts: callJsonRelativePath.split('/'),
      value: call,
      description: `Per-call output for ${call.label}`,
      publicFunction: call.publicFunction,
      monitorId: call.monitorId,
      format: call.variant,
      callId: call.id,
    })
    logCallFinished(call)
  }

  return call
}

/**
 * Executes the full smoke report run in a stable, sequential order so per-call
 * timings are easier to compare and capture APIs are not stressed in parallel.
 */
async function runApiCoverage(context: ReportContext): Promise<Monitor[]> {
  let monitors: Monitor[] = []

  logSection('Discovering Monitors')
  const getMonitorsCall = await recordCall(context, {
    label: 'getMonitors',
    publicFunction: 'getMonitors',
    variant: 'default',
    monitor: null,
    run: () => getMonitors(),
    summarize: async (value, activeContext) => {
      monitors = [...value]
      await writeJsonArtifact(activeContext, {
        relativePathParts: [ARTIFACTS_DIR_NAME, 'data', 'monitors.json'],
        value,
        description: 'Monitor list returned by getMonitors()',
        publicFunction: 'getMonitors',
        monitorId: null,
        format: null,
        callId: null,
      })
      return {
        kind: 'monitor-list',
        monitorCount: value.length,
        monitorTopology: describeMonitorTopology(value.length),
        monitors: value,
      }
    },
  })

  if (getMonitorsCall.status === 'error') {
    monitors = []
  }

  logProgress('monitor count', String(monitors.length))

  if (monitors.length === 0) {
    logSection('No-Monitor Coverage')
    await runNoMonitorCoverage(context)
  } else {
    logSection('Per-Monitor Coverage')
    await runPerMonitorCoverage(context, monitors)
  }

  logSection('All-Monitor Coverage')
  await runAllMonitorCoverage(context)
  logSection('Error Probes')
  await runErrorProbeCoverage(context, monitors[0] ?? null)

  return monitors
}

/**
 * Exercises ID-based APIs when the machine reports no monitors. These calls are
 * still valuable because they prove errors are captured into the report.
 */
async function runNoMonitorCoverage(context: ReportContext): Promise<void> {
  await recordCall(context, {
    label: `getMonitorById missing ${MISSING_MONITOR_ID}`,
    publicFunction: 'getMonitorById',
    variant: 'missing-monitor',
    monitor: null,
    expectedToFail: true,
    run: () => getMonitorById(MISSING_MONITOR_ID),
    summarize: summarizeMonitor,
  })

  for (const format of ALL_FORMATS) {
    await recordCall(context, {
      label: `captureMonitor missing ${MISSING_MONITOR_ID} ${format}`,
      publicFunction: 'captureMonitor',
      variant: format,
      monitor: null,
      expectedToFail: true,
      run: () => captureMonitor(MISSING_MONITOR_ID, format),
      summarize: summarizeBufferCaptureResult,
    })
  }

  for (const format of ENCODED_FORMATS) {
    await recordCall(context, {
      label: `captureMonitorBase64 missing ${MISSING_MONITOR_ID} ${format}`,
      publicFunction: 'captureMonitorBase64',
      variant: format,
      monitor: null,
      expectedToFail: true,
      run: () => captureMonitorBase64(MISSING_MONITOR_ID, format),
      summarize: summarizeBase64CaptureResult,
    })
  }
}

/**
 * Exercises per-monitor APIs and writes artifacts that can be reviewed beside
 * the monitor metadata that produced them.
 */
async function runPerMonitorCoverage(context: ReportContext, monitors: readonly Monitor[]): Promise<void> {
  for (const monitor of monitors) {
    logProgress('monitor', `${monitor.id} / ${displayMonitorName(monitor)}`)

    await recordCall(context, {
      label: `getMonitorById ${monitor.id}`,
      publicFunction: 'getMonitorById',
      variant: 'existing-monitor',
      monitor,
      run: () => getMonitorById(monitor.id),
      summarize: summarizeMonitor,
    })

    await recordCall(context, {
      label: `captureMonitor ${monitor.id} default`,
      publicFunction: 'captureMonitor',
      variant: 'default (Png)',
      monitor,
      run: () => captureMonitor(monitor.id),
      summarize: summarizeBufferCaptureResult,
    })

    for (const format of ALL_FORMATS) {
      await recordCall(context, {
        label: `captureMonitor ${monitor.id} ${format}`,
        publicFunction: 'captureMonitor',
        variant: format,
        monitor,
        run: () => captureMonitor(monitor.id, format),
        summarize: summarizeBufferCaptureResult,
      })
    }

    await recordCall(context, {
      label: `captureMonitorBase64 ${monitor.id} default`,
      publicFunction: 'captureMonitorBase64',
      variant: 'default (Png)',
      monitor,
      run: () => captureMonitorBase64(monitor.id),
      summarize: summarizeBase64CaptureResult,
    })

    for (const format of ENCODED_FORMATS) {
      await recordCall(context, {
        label: `captureMonitorBase64 ${monitor.id} ${format}`,
        publicFunction: 'captureMonitorBase64',
        variant: format,
        monitor,
        run: () => captureMonitorBase64(monitor.id, format),
        summarize: summarizeBase64CaptureResult,
      })
    }
  }
}

/**
 * Exercises all-monitor APIs, including the optional default format overload and
 * every explicit supported format.
 */
async function runAllMonitorCoverage(context: ReportContext): Promise<void> {
  await recordCall(context, {
    label: 'captureAllMonitors default',
    publicFunction: 'captureAllMonitors',
    variant: 'default (Png)',
    monitor: null,
    run: () => captureAllMonitors(),
    summarize: summarizeBufferCaptureResults,
  })

  for (const format of ALL_FORMATS) {
    await recordCall(context, {
      label: `captureAllMonitors ${format}`,
      publicFunction: 'captureAllMonitors',
      variant: format,
      monitor: null,
      run: () => captureAllMonitors(format),
      summarize: summarizeBufferCaptureResults,
    })
  }

  await recordCall(context, {
    label: 'captureAllMonitorsBase64 default',
    publicFunction: 'captureAllMonitorsBase64',
    variant: 'default (Png)',
    monitor: null,
    run: () => captureAllMonitorsBase64(),
    summarize: summarizeBase64CaptureResults,
  })

  for (const format of ENCODED_FORMATS) {
    await recordCall(context, {
      label: `captureAllMonitorsBase64 ${format}`,
      publicFunction: 'captureAllMonitorsBase64',
      variant: format,
      monitor: null,
      run: () => captureAllMonitorsBase64(format),
      summarize: summarizeBase64CaptureResults,
    })
  }
}

/**
 * Records known invalid argument paths so the report includes the current public
 * error surface without making the script fail.
 */
async function runErrorProbeCoverage(context: ReportContext, monitor: Monitor | null): Promise<void> {
  const monitorId = monitor?.id ?? MISSING_MONITOR_ID

  await recordCall(context, {
    label: 'captureMonitor invalid format Bmp',
    publicFunction: 'captureMonitor',
    variant: 'invalid format Bmp',
    monitor,
    expectedToFail: true,
    run: () => captureMonitor(monitorId, 'Bmp'),
    summarize: summarizeBufferCaptureResult,
  })

  await recordCall(context, {
    label: 'captureAllMonitors invalid format Tiff',
    publicFunction: 'captureAllMonitors',
    variant: 'invalid format Tiff',
    monitor: null,
    expectedToFail: true,
    run: () => captureAllMonitors('Tiff'),
    summarize: summarizeBufferCaptureResults,
  })

  await recordCall(context, {
    label: 'captureMonitorBase64 Raw',
    publicFunction: 'captureMonitorBase64',
    variant: 'unsupported Raw',
    monitor,
    expectedToFail: true,
    run: () => captureMonitorBase64(monitorId, 'Raw'),
    summarize: summarizeBase64CaptureResult,
  })

  await recordCall(context, {
    label: 'captureAllMonitorsBase64 Raw',
    publicFunction: 'captureAllMonitorsBase64',
    variant: 'unsupported Raw',
    monitor: null,
    expectedToFail: true,
    run: () => captureAllMonitorsBase64('Raw'),
    summarize: summarizeBase64CaptureResults,
  })

  await recordCall(context, {
    label: 'captureMonitorBase64 invalid format Gif',
    publicFunction: 'captureMonitorBase64',
    variant: 'invalid format Gif',
    monitor,
    expectedToFail: true,
    run: () => captureMonitorBase64(monitorId, 'Gif'),
    summarize: summarizeBase64CaptureResult,
  })

  await recordCall(context, {
    label: 'captureAllMonitorsBase64 invalid format Tga',
    publicFunction: 'captureAllMonitorsBase64',
    variant: 'invalid format Tga',
    monitor: null,
    expectedToFail: true,
    run: () => captureAllMonitorsBase64('Tga'),
    summarize: summarizeBase64CaptureResults,
  })
}

/**
 * Summarizes a monitor returned by getMonitorById and stores it separately for
 * easy comparison with the original getMonitors output.
 */
async function summarizeMonitor(value: Monitor, context: ReportContext, call: CallRecord): Promise<CallResultSummary> {
  await writeJsonArtifact(context, {
    relativePathParts: [ARTIFACTS_DIR_NAME, 'data', `${call.id}-monitor.json`],
    value,
    description: `Monitor metadata returned by ${call.label}`,
    publicFunction: call.publicFunction,
    monitorId: value.id,
    format: null,
    callId: call.id,
  })

  return {
    kind: 'monitor',
    monitor: value,
  }
}

/**
 * Stores a single Buffer-backed capture result and returns metadata suitable for
 * JSON and HTML rendering.
 */
async function summarizeBufferCaptureResult(
  value: CaptureResult,
  context: ReportContext,
  call: CallRecord,
): Promise<CallResultSummary> {
  const capture = await storeBufferCapture(value, context, call)
  return {
    kind: 'capture-result',
    capture,
  }
}

/**
 * Stores a list of Buffer-backed capture results returned by captureAllMonitors.
 */
async function summarizeBufferCaptureResults(
  value: CaptureResult[],
  context: ReportContext,
  call: CallRecord,
): Promise<CallResultSummary> {
  const captures: StoredCapture[] = []
  for (const result of value) {
    captures.push(await storeBufferCapture(result, context, call))
  }

  return {
    kind: 'capture-result-list',
    monitorCount: value.length,
    monitorTopology: describeMonitorTopology(value.length),
    captures,
  }
}

/**
 * Stores a single Base64-backed capture result as both the original Base64 text
 * and a decoded image file for browser preview.
 */
async function summarizeBase64CaptureResult(
  value: Base64CaptureResult,
  context: ReportContext,
  call: CallRecord,
): Promise<CallResultSummary> {
  const capture = await storeBase64Capture(value, context, call)
  return {
    kind: 'base64-capture-result',
    capture,
  }
}

/**
 * Stores a list of Base64-backed capture results returned by
 * captureAllMonitorsBase64.
 */
async function summarizeBase64CaptureResults(
  value: Base64CaptureResult[],
  context: ReportContext,
  call: CallRecord,
): Promise<CallResultSummary> {
  const captures: StoredCapture[] = []
  for (const result of value) {
    captures.push(await storeBase64Capture(result, context, call))
  }

  return {
    kind: 'base64-capture-result-list',
    monitorCount: value.length,
    monitorTopology: describeMonitorTopology(value.length),
    captures,
  }
}

/**
 * Persists raw or encoded Buffer screenshot bytes and records file metadata.
 */
async function storeBufferCapture(
  result: CaptureResult,
  context: ReportContext,
  call: CallRecord,
): Promise<StoredCapture> {
  const format = String(result.screenshot.format)
  const bytes = ensureBuffer(result.screenshot.data, `${call.label} screenshot.data`)
  const monitorSegment = monitorFileSegment(result.monitor)
  const extension = extensionForFormat(format)
  const fileName = `${call.id}-${monitorSegment}-${slug(format)}.${extension}`
  const isRaw = format === 'Raw'
  const relativePathParts = isRaw
    ? [ARTIFACTS_DIR_NAME, 'raw', fileName]
    : [ARTIFACTS_DIR_NAME, 'images', fileName]
  const artifact = await writeArtifact(context, {
    kind: isRaw ? 'raw-rgba' : 'image',
    relativePathParts,
    data: bytes,
    description: `${call.label} screenshot bytes`,
    mimeType: mimeTypeForFormat(format),
    publicFunction: call.publicFunction,
    monitorId: result.monitor.id,
    format,
    callId: call.id,
  })

  return buildStoredCapture({
    source: call.publicFunction,
    callId: call.id,
    monitor: result.monitor,
    size: result.screenshot.size,
    format,
    dataKind: 'Buffer',
    byteLength: bytes.byteLength,
    base64Length: null,
    sha256: artifact.sha256,
    imageRelativePath: isRaw ? null : artifact.relativePath,
    rawRelativePath: isRaw ? artifact.relativePath : null,
    base64RelativePath: null,
  })
}

/**
 * Persists the Base64 string and a decoded binary image for each Base64 capture.
 */
async function storeBase64Capture(
  result: Base64CaptureResult,
  context: ReportContext,
  call: CallRecord,
): Promise<StoredCapture> {
  const format = String(result.screenshot.format)
  const base64Data = ensureString(result.screenshot.data, `${call.label} screenshot.data`)
  const decoded = Buffer.from(base64Data, 'base64')
  const monitorSegment = monitorFileSegment(result.monitor)
  const extension = extensionForFormat(format)
  const baseName = `${call.id}-${monitorSegment}-${slug(format)}`

  const base64Artifact = await writeArtifact(context, {
    kind: 'base64-text',
    relativePathParts: [ARTIFACTS_DIR_NAME, 'base64', `${baseName}.base64.txt`],
    data: base64Data,
    description: `${call.label} original Base64 string`,
    mimeType: 'text/plain; charset=utf-8',
    publicFunction: call.publicFunction,
    monitorId: result.monitor.id,
    format,
    callId: call.id,
  })

  const imageArtifact = await writeArtifact(context, {
    kind: 'image',
    relativePathParts: [ARTIFACTS_DIR_NAME, 'images', `${baseName}-decoded.${extension}`],
    data: decoded,
    description: `${call.label} decoded Base64 image`,
    mimeType: mimeTypeForFormat(format),
    publicFunction: call.publicFunction,
    monitorId: result.monitor.id,
    format,
    callId: call.id,
  })

  return buildStoredCapture({
    source: call.publicFunction,
    callId: call.id,
    monitor: result.monitor,
    size: result.screenshot.size,
    format,
    dataKind: 'Base64',
    byteLength: decoded.byteLength,
    base64Length: base64Data.length,
    sha256: imageArtifact.sha256,
    imageRelativePath: imageArtifact.relativePath,
    rawRelativePath: null,
    base64RelativePath: base64Artifact.relativePath,
  })
}

/**
 * Builds capture metadata that relates screenshot size, monitor physical bounds,
 * raw-byte expectations, and saved artifact paths.
 */
function buildStoredCapture(input: {
  source: PublicFunctionName
  callId: string
  monitor: Monitor
  size: Size
  format: string
  dataKind: 'Buffer' | 'Base64'
  byteLength: number
  base64Length: number | null
  sha256: string
  imageRelativePath: string | null
  rawRelativePath: string | null
  base64RelativePath: string | null
}): StoredCapture {
  const pixelCount = getPixelCount(input.size)
  const expectedRawByteLength = input.format === 'Raw' && pixelCount !== null ? pixelCount * 4 : null
  const rawByteLengthMatchesExpected = expectedRawByteLength === null ? null : input.byteLength === expectedRawByteLength
  const matchesMonitorPhysicalSize = monitorPhysicalSizeMatches(input.monitor, input.size)

  return {
    source: input.source,
    callId: input.callId,
    monitor: input.monitor,
    screenshot: {
      dataKind: input.dataKind,
      format: input.format,
      mimeType: mimeTypeForFormat(input.format),
      size: input.size,
      pixelCount,
      byteLength: input.byteLength,
      base64Length: input.base64Length,
      sha256: input.sha256,
      expectedRawByteLength,
      rawByteLengthMatchesExpected,
      matchesMonitorPhysicalSize,
      imageRelativePath: input.imageRelativePath,
      rawRelativePath: input.rawRelativePath,
      base64RelativePath: input.base64RelativePath,
    },
  }
}

/**
 * Writes a JSON artifact using stable indentation and registers it in the
 * report manifest.
 */
async function writeJsonArtifact(
  context: ReportContext,
  options: {
    relativePathParts: string[]
    value: unknown
    description: string
    publicFunction: PublicFunctionName | null
    monitorId: number | null
    format: string | null
    callId: string | null
  },
): Promise<ArtifactRecord> {
  return writeArtifact(context, {
    kind: 'json',
    relativePathParts: options.relativePathParts,
    data: `${JSON.stringify(options.value, null, 2)}\n`,
    description: options.description,
    mimeType: 'application/json; charset=utf-8',
    publicFunction: options.publicFunction,
    monitorId: options.monitorId,
    format: options.format,
    callId: options.callId,
  })
}

/**
 * Writes any text or binary artifact and records size plus SHA-256 checksum so
 * reviewers can compare files across runs.
 */
async function writeArtifact(
  context: ReportContext,
  options: {
    kind: ArtifactKind
    relativePathParts: string[]
    data: string | Buffer
    description: string
    mimeType: string | null
    publicFunction: PublicFunctionName | null
    monitorId: number | null
    format: string | null
    callId: string | null
  },
): Promise<ArtifactRecord> {
  const absolutePath = resolve(context.runDir, ...options.relativePathParts)
  const relativePath = toPosixRelativePath(context.runDir, absolutePath)
  if (relativePath.startsWith('..')) {
    throw new Error(`Refusing to write artifact outside report directory: ${absolutePath}`)
  }

  await mkdir(dirname(absolutePath), { recursive: true })
  await writeFile(absolutePath, options.data)

  const artifact: ArtifactRecord = {
    kind: options.kind,
    relativePath,
    bytes: byteLengthOf(options.data),
    sha256: sha256(options.data),
    description: options.description,
    mimeType: options.mimeType,
    publicFunction: options.publicFunction,
    monitorId: options.monitorId,
    format: options.format,
    callId: options.callId,
  }

  context.artifacts.push(artifact)
  return artifact
}

/**
 * Reads package.json for report context while avoiding any hard dependency on a
 * particular package.json shape.
 */
async function readPackageMetadata(projectRoot: string): Promise<PackageMetadata> {
  try {
    const raw = await readFile(resolve(projectRoot, 'package.json'), 'utf8')
    const parsed: unknown = JSON.parse(raw)
    const record = isRecord(parsed) ? parsed : {}
    const engines = isRecord(record.engines) ? record.engines : {}

    return {
      name: typeof record.name === 'string' ? record.name : null,
      version: typeof record.version === 'string' ? record.version : null,
      nodeEngine: typeof engines.node === 'string' ? engines.node : null,
      readError: null,
    }
  } catch (error) {
    return {
      name: null,
      version: null,
      nodeEngine: null,
      readError: serializeError(error),
    }
  }
}

/**
 * Captures stable environment facts that are useful when comparing report runs.
 */
function captureEnvironment(): EnvironmentSnapshot {
  const cpuList = cpus()
  return {
    nodeVersion: process.version,
    nodeVersions: process.versions,
    typescriptRuntimeSupport: getProcessTypeScriptFeature(),
    platform: process.platform,
    arch: process.arch,
    pid: process.pid,
    cwd: process.cwd(),
    hostName: hostname(),
    osType: osType(),
    osRelease: release(),
    osUptimeSeconds: osUptime(),
    cpuCount: cpuList.length,
    availableParallelism: availableParallelism(),
    totalMemoryBytes: totalmem(),
    freeMemoryBytes: freemem(),
    processUptimeSeconds: process.uptime(),
    memoryUsage: process.memoryUsage(),
  }
}

/**
 * Builds the final report object that backs both summary.json and index.html.
 */
async function buildFinalReport(context: ReportContext, monitors: Monitor[]): Promise<FinalReport> {
  const completedAt = new Date()
  const reportHtmlPath = resolve(context.runDir, 'index.html')
  const calls = context.calls
  const expectedErrorCount = calls.filter((call) => call.expectedToFail && call.status === 'error').length
  const artifactBytes = context.artifacts.reduce((total, artifact) => total + artifact.bytes, 0)

  return {
    generatedAtIso: context.reportStartedAt.toISOString(),
    completedAtIso: completedAt.toISOString(),
    durationMs: completedAt.getTime() - context.reportStartedAt.getTime(),
    reportRootName: REPORTS_ROOT_NAME,
    reportDirectory: context.runDir,
    reportUrl: pathToFileURL(reportHtmlPath).href,
    package: await readPackageMetadata(context.projectRoot),
    environment: captureEnvironment(),
    monitorTopology: describeMonitorTopology(monitors.length),
    monitorCount: monitors.length,
    monitors,
    functionsCovered: [...PUBLIC_FUNCTIONS],
    formatsCovered: [...ALL_FORMATS],
    encodedFormatsCovered: [...ENCODED_FORMATS],
    totals: {
      calls: calls.length,
      successes: calls.filter((call) => call.status === 'success').length,
      errors: calls.filter((call) => call.status === 'error').length,
      expectedErrors: expectedErrorCount,
      artifacts: context.artifacts.length,
      artifactBytes,
      captures: collectCaptures(calls).length,
    },
    calls,
    artifacts: [...context.artifacts],
    aggregates: {
      byFunction: buildTimingAggregates(calls, context.artifacts, (call) => call.publicFunction),
      byFunctionAndVariant: buildTimingAggregates(calls, context.artifacts, (call) => `${call.publicFunction} / ${call.variant}`),
      byMonitor: buildTimingAggregates(calls, context.artifacts, (call) => call.monitorName ?? 'all monitors or no monitor'),
    },
  }
}

/**
 * Builds timing aggregates used by the report tables.
 */
function buildTimingAggregates(
  calls: readonly CallRecord[],
  artifacts: readonly ArtifactRecord[],
  keyForCall: (call: CallRecord) => string,
): TimingAggregate[] {
  const artifactsByPath = new Map(artifacts.map((artifact) => [artifact.relativePath, artifact]))
  const aggregates = new Map<string, {
    key: string
    callCount: number
    successCount: number
    errorCount: number
    expectedErrorCount: number
    totalDurationMs: number
    minDurationMs: number
    maxDurationMs: number
    totalArtifactBytes: number
  }>()

  for (const call of calls) {
    const key = keyForCall(call)
    const artifactBytes = call.artifactRelativePaths.reduce((total, relativePath) => {
      const artifact = artifactsByPath.get(relativePath)
      return total + (artifact?.bytes ?? 0)
    }, 0)
    const aggregate = aggregates.get(key) ?? {
      key,
      callCount: 0,
      successCount: 0,
      errorCount: 0,
      expectedErrorCount: 0,
      totalDurationMs: 0,
      minDurationMs: Number.POSITIVE_INFINITY,
      maxDurationMs: 0,
      totalArtifactBytes: 0,
    }

    aggregate.callCount += 1
    aggregate.successCount += call.status === 'success' ? 1 : 0
    aggregate.errorCount += call.status === 'error' ? 1 : 0
    aggregate.expectedErrorCount += call.expectedToFail && call.status === 'error' ? 1 : 0
    aggregate.totalDurationMs += call.durationMs
    aggregate.minDurationMs = Math.min(aggregate.minDurationMs, call.durationMs)
    aggregate.maxDurationMs = Math.max(aggregate.maxDurationMs, call.durationMs)
    aggregate.totalArtifactBytes += artifactBytes

    aggregates.set(key, aggregate)
  }

  return [...aggregates.values()]
    .map((aggregate) => ({
      key: aggregate.key,
      callCount: aggregate.callCount,
      successCount: aggregate.successCount,
      errorCount: aggregate.errorCount,
      expectedErrorCount: aggregate.expectedErrorCount,
      totalDurationMs: aggregate.totalDurationMs,
      averageDurationMs: aggregate.callCount === 0 ? 0 : aggregate.totalDurationMs / aggregate.callCount,
      minDurationMs: Number.isFinite(aggregate.minDurationMs) ? aggregate.minDurationMs : 0,
      maxDurationMs: aggregate.maxDurationMs,
      totalArtifactBytes: aggregate.totalArtifactBytes,
    }))
    .sort((left, right) => right.totalDurationMs - left.totalDurationMs)
}

/**
 * Renders the self-contained HTML report.
 */
function renderHtmlReport(report: FinalReport): string {
  const captures = collectCaptures(report.calls)
  const errors = report.calls.filter((call) => call.status === 'error')

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>captura Smoke Report</title>
  <style>
${renderReportCss()}
  </style>
</head>
<body>
  <header class="hero">
    <div>
      <p class="eyebrow">captura smoke report</p>
      <h1>${escapeHtml(report.package.name ?? 'captura')} ${escapeHtml(report.package.version ?? 'unknown version')}</h1>
      <p class="hero-subtitle">Generated ${escapeHtml(formatDateTime(report.generatedAtIso))} in ${escapeHtml(formatMs(report.durationMs))}</p>
    </div>
    <div class="hero-panel">
      <div class="hero-panel-label">Report directory</div>
      <code>${escapeHtml(report.reportDirectory)}</code>
    </div>
  </header>

  <main>
    <section class="metric-grid" aria-label="Run overview">
      ${renderMetric('Monitor state', report.monitorTopology)}
      ${renderMetric('API calls', String(report.totals.calls))}
      ${renderMetric('Recorded captures', String(report.totals.captures))}
      ${renderMetric('Artifacts', `${report.totals.artifacts} files / ${formatBytes(report.totals.artifactBytes)}`)}
      ${renderMetric('Errors recorded', `${report.totals.errors} total, ${report.totals.expectedErrors} expected`)}
      ${renderMetric('Runtime', `${report.environment.nodeVersion} / ${report.environment.platform}-${report.environment.arch}`)}
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">coverage</p>
        <h2>Public API and Variants</h2>
      </div>
      <div class="coverage-grid">
        <div class="panel">
          <h3>Functions</h3>
          <div class="pill-row">${report.functionsCovered.map((name) => `<span class="pill neutral">${escapeHtml(name)}</span>`).join('')}</div>
        </div>
        <div class="panel">
          <h3>Buffer formats</h3>
          <div class="pill-row">${report.formatsCovered.map((name) => `<span class="pill blue">${escapeHtml(name)}</span>`).join('')}</div>
        </div>
        <div class="panel">
          <h3>Base64 formats</h3>
          <div class="pill-row">${report.encodedFormatsCovered.map((name) => `<span class="pill green">${escapeHtml(name)}</span>`).join('')}</div>
        </div>
      </div>
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">environment</p>
        <h2>Run Context</h2>
      </div>
      ${renderEnvironment(report)}
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">monitors</p>
        <h2>Monitor Metadata and Related Captures</h2>
      </div>
      ${renderMonitorSections(report.monitors, captures)}
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">timings</p>
        <h2>Timing Summary</h2>
      </div>
      <div class="table-stack">
        ${renderAggregateTable('By function', report.aggregates.byFunction)}
        ${renderAggregateTable('By function and variant', report.aggregates.byFunctionAndVariant)}
        ${renderAggregateTable('By monitor scope', report.aggregates.byMonitor)}
      </div>
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">operation log</p>
        <h2>Every API Call</h2>
      </div>
      ${renderCallsTable(report.calls)}
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">errors</p>
        <h2>Recorded Errors</h2>
      </div>
      ${renderErrors(errors)}
    </section>

    <section class="section">
      <div class="section-heading">
        <p class="eyebrow">files</p>
        <h2>Artifact Index</h2>
      </div>
      ${renderArtifacts(report.artifacts)}
    </section>

  </main>
</body>
</html>
`
}

/**
 * CSS is kept inline so a report directory can be moved or archived as one
 * self-contained folder.
 */
function renderReportCss(): string {
  return `
    :root {
      color-scheme: light;
      --paper: #f6f3ed;
      --surface: #ffffff;
      --surface-soft: #fbfaf7;
      --ink: #172033;
      --muted: #5f6978;
      --line: #ded8cd;
      --blue: #2457c5;
      --green: #087a61;
      --amber: #a15c0b;
      --red: #b42318;
      --violet: #6f3fb5;
      --shadow: 0 18px 45px rgba(23, 32, 51, 0.08);
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      min-width: 320px;
      background: var(--paper);
      color: var(--ink);
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      line-height: 1.5;
    }

    a { color: var(--blue); text-decoration-thickness: 1px; text-underline-offset: 2px; }
    code { font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace; font-size: 0.9em; }

    .hero {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(260px, 440px);
      gap: 24px;
      align-items: end;
      padding: 44px clamp(20px, 5vw, 72px) 34px;
      background: #172033;
      color: #fffdf8;
      border-bottom: 6px solid #c9a227;
    }

    .hero h1 {
      margin: 4px 0 10px;
      font-size: clamp(2rem, 5vw, 4.5rem);
      line-height: 0.98;
      letter-spacing: 0;
    }

    .hero-subtitle { max-width: 760px; margin: 0; color: #d9e0e8; font-size: 1.05rem; }
    .hero-panel { padding: 18px; border: 1px solid rgba(255, 255, 255, 0.22); border-radius: 8px; background: rgba(255, 255, 255, 0.08); }
    .hero-panel-label { margin-bottom: 8px; color: #d9e0e8; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0; font-weight: 700; }
    .hero-panel code { display: block; overflow-wrap: anywhere; color: #fffdf8; }

    main { width: min(1440px, 100%); margin: 0 auto; padding: 28px clamp(16px, 4vw, 48px) 56px; }

    .eyebrow { margin: 0; color: var(--muted); font-size: 0.76rem; text-transform: uppercase; letter-spacing: 0; font-weight: 800; }
    .section { margin-top: 34px; }
    .section-heading { display: flex; flex-direction: column; gap: 4px; margin-bottom: 14px; }
    .section-heading h2 { margin: 0; font-size: 1.45rem; letter-spacing: 0; }

    .metric-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; }
    .metric, .panel { background: var(--surface); border: 1px solid var(--line); border-radius: 8px; box-shadow: var(--shadow); }
    .metric { padding: 18px; }
    .metric-label { color: var(--muted); font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0; }
    .metric-value { margin-top: 6px; font-size: 1.18rem; font-weight: 800; overflow-wrap: anywhere; }

    .coverage-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; }
    .panel { padding: 18px; }
    .panel h3 { margin: 0 0 12px; font-size: 1rem; }
    .pill-row { display: flex; flex-wrap: wrap; gap: 8px; }
    .pill { display: inline-flex; align-items: center; min-height: 28px; padding: 4px 9px; border-radius: 999px; font-size: 0.82rem; font-weight: 800; }
    .pill.neutral { background: #ece7dd; color: #3c4350; }
    .pill.blue { background: #dce7ff; color: var(--blue); }
    .pill.green { background: #dff5ec; color: var(--green); }
    .pill.amber { background: #fff0d2; color: var(--amber); }
    .pill.red { background: #ffe1dd; color: var(--red); }

    .kv-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
    .kv { padding: 12px; background: var(--surface); border: 1px solid var(--line); border-radius: 8px; }
    .kv-label { display: block; color: var(--muted); font-size: 0.76rem; font-weight: 800; text-transform: uppercase; letter-spacing: 0; }
    .kv-value { display: block; margin-top: 4px; overflow-wrap: anywhere; font-weight: 650; }

    .monitor-stack { display: grid; gap: 18px; }
    .monitor { background: var(--surface); border: 1px solid var(--line); border-radius: 8px; box-shadow: var(--shadow); overflow: hidden; }
    .monitor-header { display: flex; justify-content: space-between; gap: 16px; padding: 18px; background: var(--surface-soft); border-bottom: 1px solid var(--line); }
    .monitor-title h3 { margin: 2px 0 4px; font-size: 1.18rem; }
    .monitor-flags { display: flex; flex-wrap: wrap; gap: 8px; align-content: flex-start; justify-content: flex-end; }
    .monitor-body { padding: 18px; display: grid; gap: 18px; }
    .capture-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; }
    .capture { border: 1px solid var(--line); border-radius: 8px; background: #fff; overflow: hidden; }
    .capture-preview { display: grid; min-height: 180px; place-items: center; background: #ebe7df; border-bottom: 1px solid var(--line); }
    .capture-preview img { display: block; width: 100%; max-height: 360px; object-fit: contain; background: #111827; }
    .raw-preview { padding: 20px; color: var(--muted); text-align: center; font-weight: 700; }
    .capture-body { padding: 14px; }
    .capture-title { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 10px; }
    .capture-title h4 { margin: 0; font-size: 0.96rem; }
    .mini-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px; font-size: 0.86rem; }
    .mini-grid div { padding: 7px; background: var(--surface-soft); border-radius: 6px; overflow-wrap: anywhere; }
    .file-links { margin-top: 10px; display: flex; flex-wrap: wrap; gap: 8px; font-size: 0.86rem; }

    .table-wrap { overflow-x: auto; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); box-shadow: var(--shadow); }
    .table-stack { display: grid; gap: 16px; }
    table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
    th, td { padding: 10px 12px; border-bottom: 1px solid var(--line); text-align: left; vertical-align: top; }
    th { background: var(--surface-soft); color: var(--muted); font-size: 0.76rem; text-transform: uppercase; letter-spacing: 0; }
    tr:last-child td { border-bottom: 0; }
    td code { overflow-wrap: anywhere; }
    .muted { color: var(--muted); }
    .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }

    .status { display: inline-flex; min-height: 26px; align-items: center; border-radius: 999px; padding: 3px 9px; font-size: 0.78rem; font-weight: 800; }
    .status.success { background: #dff5ec; color: var(--green); }
    .status.error { background: #ffe1dd; color: var(--red); }
    .status.expected { background: #fff0d2; color: var(--amber); }

    .empty { padding: 22px; color: var(--muted); background: var(--surface); border: 1px dashed var(--line); border-radius: 8px; }
    .error-list { display: grid; gap: 12px; }
    .error-item { padding: 14px; background: #fff7f5; border: 1px solid #ffd1c9; border-radius: 8px; }
    .error-item h3 { margin: 0 0 8px; font-size: 1rem; }
    .error-item pre { white-space: pre-wrap; overflow-wrap: anywhere; margin: 10px 0 0; color: #6c1d16; font-size: 0.82rem; }

    @media (max-width: 980px) {
      .hero { grid-template-columns: 1fr; }
      .metric-grid, .coverage-grid, .kv-grid, .capture-grid { grid-template-columns: 1fr; }
      .monitor-header { flex-direction: column; }
      .monitor-flags { justify-content: flex-start; }
    }
  `
}

/**
 * Renders a compact overview metric.
 */
function renderMetric(label: string, value: string): string {
  return `<div class="metric"><div class="metric-label">${escapeHtml(label)}</div><div class="metric-value">${escapeHtml(value)}</div></div>`
}

/**
 * Renders environment and package metadata in a scan-friendly key/value grid.
 */
function renderEnvironment(report: FinalReport): string {
  const packageName = report.package.name ?? 'unknown'
  const packageVersion = report.package.version ?? 'unknown'
  const nodeEngine = report.package.nodeEngine ?? 'unknown'

  return `<div class="kv-grid">
    ${renderKeyValue('Package', `${packageName}@${packageVersion}`)}
    ${renderKeyValue('Node engine', nodeEngine)}
    ${renderKeyValue('Node runtime', report.environment.nodeVersion)}
    ${renderKeyValue('TypeScript runtime', String(report.environment.typescriptRuntimeSupport ?? 'unknown'))}
    ${renderKeyValue('Platform', `${report.environment.platform}-${report.environment.arch}`)}
    ${renderKeyValue('OS', `${report.environment.osType} ${report.environment.osRelease}`)}
    ${renderKeyValue('Host', report.environment.hostName)}
    ${renderKeyValue('PID', String(report.environment.pid))}
    ${renderKeyValue('CPU cores', `${report.environment.cpuCount} logical / ${report.environment.availableParallelism} available`)}
    ${renderKeyValue('System memory', `${formatBytes(report.environment.freeMemoryBytes)} free / ${formatBytes(report.environment.totalMemoryBytes)} total`)}
    ${renderKeyValue('Process RSS', formatBytes(report.environment.memoryUsage.rss))}
    ${renderKeyValue('Working directory', report.environment.cwd)}
  </div>`
}

/**
 * Renders one key/value cell.
 */
function renderKeyValue(label: string, value: string): string {
  return `<div class="kv"><span class="kv-label">${escapeHtml(label)}</span><span class="kv-value">${escapeHtml(value)}</span></div>`
}

/**
 * Renders monitor metadata and every capture related to each monitor.
 */
function renderMonitorSections(monitors: readonly Monitor[], captures: readonly StoredCapture[]): string {
  if (monitors.length === 0) {
    return '<div class="empty">No monitors were returned by getMonitors(). API errors and all-monitor calls are still recorded below.</div>'
  }

  return `<div class="monitor-stack">${monitors.map((monitor) => renderMonitorSection(monitor, captures)).join('')}</div>`
}

/**
 * Renders one monitor and filters capture artifacts by monitor id.
 */
function renderMonitorSection(monitor: Monitor, captures: readonly StoredCapture[]): string {
  const relatedCaptures = captures.filter((capture) => capture.monitor.id === monitor.id)
  return `<article class="monitor">
    <div class="monitor-header">
      <div class="monitor-title">
        <p class="eyebrow">monitor ${escapeHtml(String(monitor.id))}</p>
        <h3>${escapeHtml(displayMonitorName(monitor))}</h3>
        <div>${escapeHtml(monitor.name)}</div>
      </div>
      <div class="monitor-flags">
        ${monitor.isPrimary ? '<span class="pill green">primary</span>' : '<span class="pill neutral">not primary</span>'}
        ${monitor.isBuiltin ? '<span class="pill blue">built-in</span>' : '<span class="pill neutral">external or unknown</span>'}
      </div>
    </div>
    <div class="monitor-body">
      <div class="kv-grid">
        ${renderKeyValue('Friendly name', monitor.friendlyName)}
        ${renderKeyValue('Physical', `${monitor.physical.x}, ${monitor.physical.y}, ${monitor.physical.width} x ${monitor.physical.height}`)}
        ${renderKeyValue('Logical', `${monitor.logical.x}, ${monitor.logical.y}, ${monitor.logical.width} x ${monitor.logical.height}`)}
        ${renderKeyValue('Scale factor', String(monitor.scaleFactor))}
        ${renderKeyValue('Rotation', `${monitor.rotation} deg`)}
        ${renderKeyValue('Frequency', `${monitor.frequency} Hz`)}
        ${renderKeyValue('Primary', String(monitor.isPrimary))}
        ${renderKeyValue('Built-in', String(monitor.isBuiltin))}
      </div>
      <div class="capture-grid">
        ${relatedCaptures.length === 0 ? '<div class="empty">No captures were related to this monitor.</div>' : relatedCaptures.map(renderCaptureCard).join('')}
      </div>
    </div>
  </article>`
}

/**
 * Renders a capture artifact card with preview, dimensions, checksums, and links.
 */
function renderCaptureCard(capture: StoredCapture): string {
  const imagePath = capture.screenshot.imageRelativePath
  const rawPath = capture.screenshot.rawRelativePath
  const base64Path = capture.screenshot.base64RelativePath
  const preview = imagePath === null
    ? '<div class="raw-preview">Raw RGBA bytes saved for external pixel inspection</div>'
    : `<a href="${escapeHtml(urlPath(imagePath))}"><img loading="lazy" src="${escapeHtml(urlPath(imagePath))}" alt="${escapeHtml(capture.source)} ${escapeHtml(capture.screenshot.format)} capture for monitor ${escapeHtml(String(capture.monitor.id))}"></a>`

  const links = [
    imagePath === null ? null : `<a href="${escapeHtml(urlPath(imagePath))}">image</a>`,
    rawPath === null ? null : `<a href="${escapeHtml(urlPath(rawPath))}">raw bytes</a>`,
    base64Path === null ? null : `<a href="${escapeHtml(urlPath(base64Path))}">base64 text</a>`,
  ].filter((value): value is string => value !== null)

  return `<article class="capture">
    <div class="capture-preview">${preview}</div>
    <div class="capture-body">
      <div class="capture-title">
        <h4>${escapeHtml(capture.source)}</h4>
        <span class="pill ${capture.screenshot.dataKind === 'Base64' ? 'green' : 'blue'}">${escapeHtml(capture.screenshot.format)}</span>
      </div>
      <div class="mini-grid">
        <div><strong>Size</strong><br>${escapeHtml(`${capture.screenshot.size.width} x ${capture.screenshot.size.height}`)}</div>
        <div><strong>Payload</strong><br>${escapeHtml(formatBytes(capture.screenshot.byteLength))}</div>
        <div><strong>Physical match</strong><br>${escapeHtml(displayNullableBoolean(capture.screenshot.matchesMonitorPhysicalSize))}</div>
        <div><strong>Raw length match</strong><br>${escapeHtml(displayNullableBoolean(capture.screenshot.rawByteLengthMatchesExpected))}</div>
        <div><strong>MIME</strong><br>${escapeHtml(capture.screenshot.mimeType)}</div>
        <div><strong>SHA-256</strong><br><code>${escapeHtml(shortHash(capture.screenshot.sha256))}</code></div>
      </div>
      <div class="file-links">${links.join('')}</div>
    </div>
  </article>`
}

/**
 * Renders an aggregate timing table.
 */
function renderAggregateTable(title: string, aggregates: readonly TimingAggregate[]): string {
  if (aggregates.length === 0) {
    return `<div class="empty">No timing data for ${escapeHtml(title)}.</div>`
  }

  return `<div class="table-wrap">
    <table>
      <caption class="sr-only">${escapeHtml(title)}</caption>
      <thead><tr><th>${escapeHtml(title)}</th><th>Calls</th><th>Success</th><th>Errors</th><th>Total</th><th>Average</th><th>Min</th><th>Max</th><th>Artifacts</th></tr></thead>
      <tbody>
        ${aggregates.map((aggregate) => `<tr>
          <td>${escapeHtml(aggregate.key)}</td>
          <td>${aggregate.callCount}</td>
          <td>${aggregate.successCount}</td>
          <td>${aggregate.errorCount} (${aggregate.expectedErrorCount} expected)</td>
          <td>${escapeHtml(formatMs(aggregate.totalDurationMs))}</td>
          <td>${escapeHtml(formatMs(aggregate.averageDurationMs))}</td>
          <td>${escapeHtml(formatMs(aggregate.minDurationMs))}</td>
          <td>${escapeHtml(formatMs(aggregate.maxDurationMs))}</td>
          <td>${escapeHtml(formatBytes(aggregate.totalArtifactBytes))}</td>
        </tr>`).join('')}
      </tbody>
    </table>
  </div>`
}

/**
 * Renders the full chronological operation log.
 */
function renderCallsTable(calls: readonly CallRecord[]): string {
  if (calls.length === 0) {
    return '<div class="empty">No API calls were recorded.</div>'
  }

  return `<div class="table-wrap">
    <table>
      <thead><tr><th>#</th><th>Function</th><th>Variant</th><th>Monitor</th><th>Status</th><th>Duration</th><th>Output</th></tr></thead>
      <tbody>
        ${calls.map((call) => `<tr>
          <td><code>${escapeHtml(call.id)}</code></td>
          <td>${escapeHtml(call.publicFunction)}</td>
          <td>${escapeHtml(call.variant)}</td>
          <td>${escapeHtml(call.monitorName ?? 'all monitors or no monitor')}</td>
          <td>${renderStatus(call)}</td>
          <td>${escapeHtml(formatMs(call.durationMs))}</td>
          <td>${renderCallArtifacts(call)}</td>
        </tr>`).join('')}
      </tbody>
    </table>
  </div>`
}

/**
 * Renders artifact links associated with a call.
 */
function renderCallArtifacts(call: CallRecord): string {
  if (call.artifactRelativePaths.length === 0) {
    return 'none'
  }

  return call.artifactRelativePaths
    .map((relativePath) => `<a href="${escapeHtml(urlPath(relativePath))}">${escapeHtml(relativePath)}</a>`)
    .join('<br>')
}

/**
 * Renders status with expected-error distinction.
 */
function renderStatus(call: CallRecord): string {
  if (call.status === 'success') {
    return `<span class="status success">${escapeHtml(call.outcome)}</span>`
  }

  if (call.expectedToFail) {
    return `<span class="status expected">${escapeHtml(call.outcome)}</span>`
  }

  return `<span class="status error">${escapeHtml(call.outcome)}</span>`
}

/**
 * Renders recorded errors without hiding expected negative probes.
 */
function renderErrors(errors: readonly CallRecord[]): string {
  if (errors.length === 0) {
    return '<div class="empty">No errors were recorded.</div>'
  }

  return `<div class="error-list">${errors.map((call) => `<article class="error-item">
    <h3>${escapeHtml(call.label)}</h3>
    <div>${renderStatus(call)} <span>${escapeHtml(formatMs(call.durationMs))}</span></div>
    <p><strong>${escapeHtml(call.error?.name ?? 'Error')}:</strong> ${escapeHtml(call.error?.message ?? 'No message')}</p>
    ${call.error?.stack === null ? '' : `<pre>${escapeHtml(call.error?.stack ?? '')}</pre>`}
  </article>`).join('')}</div>`
}

/**
 * Renders every generated artifact with size and checksum.
 */
function renderArtifacts(artifacts: readonly ArtifactRecord[]): string {
  if (artifacts.length === 0) {
    return '<div class="empty">No artifacts were written.</div>'
  }

  return `<div class="table-wrap">
    <table>
      <thead><tr><th>Kind</th><th>File</th><th>Function</th><th>Format</th><th>Monitor</th><th>Size</th><th>SHA-256</th></tr></thead>
      <tbody>
        ${artifacts.map((artifact) => `<tr>
          <td>${escapeHtml(artifact.kind)}</td>
          <td><a href="${escapeHtml(urlPath(artifact.relativePath))}">${escapeHtml(artifact.relativePath)}</a><br><span class="muted">${escapeHtml(artifact.description)}</span></td>
          <td>${escapeHtml(artifact.publicFunction ?? 'n/a')}</td>
          <td>${escapeHtml(artifact.format ?? 'n/a')}</td>
          <td>${escapeHtml(artifact.monitorId === null ? 'n/a' : String(artifact.monitorId))}</td>
          <td>${escapeHtml(formatBytes(artifact.bytes))}</td>
          <td><code>${escapeHtml(shortHash(artifact.sha256))}</code></td>
        </tr>`).join('')}
      </tbody>
    </table>
  </div>`
}

/**
 * Collects capture summaries from all call records.
 */
function collectCaptures(calls: readonly CallRecord[]): StoredCapture[] {
  const captures: StoredCapture[] = []

  for (const call of calls) {
    if (call.result?.capture !== undefined) {
      captures.push(call.result.capture)
    }
    if (call.result?.captures !== undefined) {
      captures.push(...call.result.captures)
    }
  }

  return captures
}

/**
 * Produces a readable monitor display name with stable fallback order.
 */
function displayMonitorName(monitor: Monitor): string {
  return monitor.friendlyName.trim() || monitor.name.trim() || `Monitor ${monitor.id}`
}

/**
 * Produces a safe monitor segment for artifact file names.
 */
function monitorFileSegment(monitor: Monitor): string {
  return `monitor-${monitor.id}-${slug(displayMonitorName(monitor))}`
}

/**
 * Converts arbitrary text into a file-name-safe lowercase segment.
 */
function slug(value: string): string {
  const slugged = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 96)

  return slugged.length === 0 ? 'item' : slugged
}

/**
 * Returns a user-facing topology label for zero, one, or many monitors.
 */
function describeMonitorTopology(count: number): string {
  if (count === 0) return 'no monitors detected'
  if (count === 1) return 'single monitor detected'
  return `${count} monitors detected`
}

/**
 * Computes pixel count only when width and height are safe finite integers.
 */
function getPixelCount(size: Size): number | null {
  if (!Number.isSafeInteger(size.width) || !Number.isSafeInteger(size.height)) return null
  if (size.width <= 0 || size.height <= 0) return null
  const pixelCount = size.width * size.height
  return Number.isSafeInteger(pixelCount) ? pixelCount : null
}

/**
 * Compares screenshot dimensions with monitor physical bounds.
 */
function monitorPhysicalSizeMatches(monitor: Monitor, size: Size): boolean | null {
  if (!Number.isFinite(monitor.physical.width) || !Number.isFinite(monitor.physical.height)) return null
  if (!Number.isFinite(size.width) || !Number.isFinite(size.height)) return null
  return monitor.physical.width === size.width && monitor.physical.height === size.height
}

/**
 * Maps captura image format names to file extensions.
 */
function extensionForFormat(format: string): string {
  switch (format) {
    case 'Png':
      return 'png'
    case 'Jpeg':
      return 'jpg'
    case 'WebP':
      return 'webp'
    case 'Avif':
      return 'avif'
    case 'Raw':
      return 'rgba'
    default:
      return 'bin'
  }
}

/**
 * Maps captura image format names to MIME types for the HTML report.
 */
function mimeTypeForFormat(format: string): string {
  switch (format) {
    case 'Png':
      return 'image/png'
    case 'Jpeg':
      return 'image/jpeg'
    case 'WebP':
      return 'image/webp'
    case 'Avif':
      return 'image/avif'
    case 'Raw':
      return 'application/octet-stream'
    default:
      return 'application/octet-stream'
  }
}

/**
 * Ensures a value is a Buffer or Uint8Array before writing binary artifacts.
 */
function ensureBuffer(value: unknown, label: string): Buffer {
  if (Buffer.isBuffer(value)) return value
  if (value instanceof Uint8Array) return Buffer.from(value)
  throw new TypeError(`${label} must be a Buffer or Uint8Array`)
}

/**
 * Ensures a value is a string before writing Base64 artifacts.
 */
function ensureString(value: unknown, label: string): string {
  if (typeof value === 'string') return value
  throw new TypeError(`${label} must be a string`)
}

/**
 * Serializes unknown thrown values into JSON-safe error metadata.
 */
function serializeError(error: unknown): SerializedError {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      code: getErrorCode(error),
      stack: typeof error.stack === 'string' ? error.stack : null,
    }
  }

  return {
    name: 'NonErrorThrow',
    message: String(error),
    code: null,
    stack: null,
  }
}

/**
 * Extracts common Node-style error codes without assuming a specific class.
 */
function getErrorCode(error: unknown): string | number | null {
  if (!isRecord(error)) return null
  const code = error.code
  return typeof code === 'string' || typeof code === 'number' ? code : null
}

/**
 * Type guard for object records.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

/**
 * Reads the Node TypeScript feature flag without relying on old Node versions
 * exposing process.features.typescript.
 */
function getProcessTypeScriptFeature(): string | boolean | null {
  const features = process.features as NodeJS.ProcessFeatures & { typescript?: string | boolean }
  return features.typescript ?? null
}

/**
 * Formats bytes using binary units.
 */
function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes)) return 'unknown'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  let value = bytes
  let unitIndex = 0

  while (Math.abs(value) >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }

  const unit = units[unitIndex] ?? 'B'
  const digits = unitIndex === 0 ? 0 : 2
  return `${value.toFixed(digits)} ${unit}`
}

/**
 * Formats elapsed milliseconds with stable precision.
 */
function formatMs(milliseconds: number): string {
  if (!Number.isFinite(milliseconds)) return 'unknown'
  if (milliseconds < 1000) return `${milliseconds.toFixed(2)} ms`
  return `${(milliseconds / 1000).toFixed(2)} s`
}

/**
 * Formats an ISO date for display while preserving exact source timestamps in
 * summary JSON.
 */
function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString()
}

/**
 * Pads a number to two digits.
 */
function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

/**
 * Computes a SHA-256 digest for strings and buffers.
 */
function sha256(data: string | Buffer): string {
  return createHash('sha256').update(data).digest('hex')
}

/**
 * Returns byte length for strings and buffers using Node's Buffer semantics.
 */
function byteLengthOf(data: string | Buffer): number {
  return typeof data === 'string' ? Buffer.byteLength(data, 'utf8') : data.byteLength
}

/**
 * Shortens a checksum for table display while full hashes remain in JSON.
 */
function shortHash(hash: string): string {
  return hash.length <= 16 ? hash : `${hash.slice(0, 12)}...${hash.slice(-8)}`
}

/**
 * Converts a filesystem path to a report-relative POSIX-style path.
 */
function toPosixRelativePath(from: string, to: string): string {
  return relative(from, to).split(sep).join('/')
}

/**
 * Percent-encodes relative paths for HTML href/src attributes.
 */
function urlPath(relativePath: string): string {
  return relativePath.split('/').map(encodeURIComponent).join('/')
}

/**
 * Escapes text for safe HTML insertion.
 */
function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => {
    switch (character) {
      case '&':
        return '&amp;'
      case '<':
        return '&lt;'
      case '>':
        return '&gt;'
      case '"':
        return '&quot;'
      case "'":
        return '&#39;'
      default:
        return character
    }
  })
}

/**
 * Displays null as n/a for tri-state checks.
 */
function displayNullableBoolean(value: boolean | null): string {
  if (value === null) return 'n/a'
  return value ? 'yes' : 'no'
}

/**
 * Prints a clear section heading for long-running terminal output.
 */
function logSection(title: string): void {
  console.log(`\n== ${title} ==`)
}

/**
 * Prints one stable key/value progress line.
 */
function logProgress(label: string, value: string): void {
  console.log(`   ${label.padEnd(LOG_LABEL_WIDTH)} ${value}`)
}

/**
 * Announces an API operation before it starts so capture work never looks idle.
 */
function logCallStart(call: CallRecord): void {
  console.log(`\n${call.id} ${call.publicFunction}`)
  logProgress('variant', call.variant)
  logProgress('monitor', monitorScopeForCall(call))
  if (call.expectedToFail) {
    logProgress('expectation', 'error path should be recorded')
  }
}

/**
 * Prints the compact completion line for a recorded API operation.
 */
function logCallFinished(call: CallRecord): void {
  const status = call.status === 'success'
    ? 'ok'
    : call.expectedToFail
      ? 'expected error'
      : 'error'

  logProgress('status', `${status} in ${formatMs(call.durationMs)}`)
  logProgress('artifacts', `${call.artifactRelativePaths.length} file(s)`)
  if (call.error !== null) {
    logProgress('message', truncateOneLine(call.error.message, 150))
  }
}

/**
 * Describes the monitor scope for terminal output without dumping metadata.
 */
function monitorScopeForCall(call: CallRecord): string {
  if (call.monitorId === null) {
    return 'all monitors or no monitor'
  }

  return `${call.monitorId} / ${call.monitorName ?? 'unnamed monitor'}`
}

/**
 * Keeps terminal error lines readable while the full stack remains in JSON.
 */
function truncateOneLine(value: string, maxLength: number): string {
  const oneLine = value.replace(/\s+/g, ' ').trim()
  if (oneLine.length <= maxLength) {
    return oneLine
  }

  return `${oneLine.slice(0, Math.max(0, maxLength - 3))}...`
}

/**
 * Writes final JSON files and the HTML report after all API calls complete.
 */
async function writeFinalOutputs(context: ReportContext, monitors: Monitor[]): Promise<FinalReport> {
  logSection('Writing Final Report')
  logProgress('summary json', `${ARTIFACTS_DIR_NAME}/data/summary.json`)
  const reportBeforeSummary = await buildFinalReport(context, monitors)
  await writeJsonArtifact(context, {
    relativePathParts: [ARTIFACTS_DIR_NAME, 'data', 'summary.json'],
    value: reportBeforeSummary,
    description: 'Machine-readable final smoke report summary',
    publicFunction: null,
    monitorId: null,
    format: null,
    callId: null,
  })

  logProgress('html report', 'index.html')
  const finalReport = await buildFinalReport(context, monitors)
  await writeArtifact(context, {
    kind: 'html-report',
    relativePathParts: ['index.html'],
    data: renderHtmlReport(finalReport),
    description: 'HTML smoke report',
    mimeType: 'text/html; charset=utf-8',
    publicFunction: null,
    monitorId: null,
    format: null,
    callId: null,
  })

  logProgress('report url', finalReport.reportUrl)

  return finalReport
}

/**
 * Entrypoint. API-level failures are recorded in the report; infrastructure
 * failures, such as being unable to create files, set a non-zero exit code.
 */
async function main(): Promise<void> {
  console.log('captura smoke report')
  const context = await createReportContext()
  const monitors = await runApiCoverage(context)
  const finalReport = await writeFinalOutputs(context, monitors)

  logSection('Done')
  logProgress('report', finalReport.reportUrl)
  logProgress('directory', finalReport.reportDirectory)
  logProgress('api calls', String(finalReport.totals.calls))
  logProgress('captures', String(finalReport.totals.captures))
  logProgress('errors', `${finalReport.totals.errors} recorded / ${finalReport.totals.expectedErrors} expected`)
}

await main().catch((error: unknown) => {
  console.error('Failed to generate captura smoke report.')
  console.error(serializeError(error))
  process.exitCode = 1
})