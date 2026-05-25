// ESM wrapper around the CommonJS implementation in errors.js.
// Keeps a single source of truth for the helper logic while letting
// consumers `import { isCapturaError } from 'captura/errors'`.

import errors from './errors.js'

export const CapturaErrorCode = errors.CapturaErrorCode
export const getCapturaErrorCode = errors.getCapturaErrorCode
export const isCapturaError = errors.isCapturaError

export default errors
