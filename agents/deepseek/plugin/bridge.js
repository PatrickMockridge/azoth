// The argv, environment and JSON plumbing between a DSH tool and `python -m azoth.harness`.
//
// This file imports nothing from DeepSeek Harness, so `node --test` exercises it without a
// runtime, and the plugin that mounts it needs no node_modules.

import { spawn } from 'node:child_process'
import { delimiter } from 'node:path'

const DEFAULT_TIMEOUT_MS = 120_000
const DEFAULT_MAX_OUTPUT_BYTES = 4 * 1024 * 1024

const REFUSAL_HELP = {
  UnknownId:
    "That is not one of azoth's calculations. Call azoth_catalogue with plant words to find the right id.",
  UndefinedInput:
    'The calculation refused an input. Check azoth_spec for its unit, type and valid range.',
  HarnessInputError:
    'The calculation refused an input. Check azoth_spec for its unit, type and valid range.',
  InputTooLarge: 'The inputs are too large. Reduce the arrays or split the calculation.',
  OutputTooLarge: 'The result is too large to display. Ask for a smaller calculation.',
  DossierInvalid:
    'The dossier cannot be read. Check its sources, claims and targets against the intake schema.',
  DossierRequired:
    'This session requires an intake dossier. Call azoth_intake first, then pass it to azoth_calc.',
  TargetMismatch:
    'The dossier has no target, or more than one, for that id. Check the targets it names.',
  InputsMismatch: 'A dossier replaces inputs; pass one or the other, not both.',
  IntakeRefused:
    'The dossier refuses this target. Do not present the result as a constructible basis; ' +
    'answer the data requests first.',
}

function refusal(document) {
  const type = document?.error?.type ?? 'Error'
  const field = document?.error?.field
  const detail = document?.error?.message ?? 'unknown refusal'
  const help = REFUSAL_HELP[type]
  const where = field ? ` (input: ${field})` : ''
  const error = new Error(help ? `${help}${where}` : `${detail}${where}`)
  error.type = type
  error.field = field
  error.detail = detail
  return error
}

export function buildInvocation(tool, args, cfg) {
  if (!cfg || !cfg.python) {
    throw new Error('azoth bridge: config.python is required')
  }
  const pythonpath = [...(cfg.pythonpath ?? []), process.env.PYTHONPATH]
    .filter(Boolean)
    .join(delimiter)
  const env = {
    ...process.env,
    PYTHONPATH: pythonpath,
    PYTHONDONTWRITEBYTECODE: '1',
  }
  const base = [cfg.python, '-B', '-m', 'azoth.harness']
  let argv
  let stdin
  if (tool === 'run') {
    argv = [...base, 'run', String(args.id)]
    if (args.dossier_path) {
      argv.push('--dossier-file', String(args.dossier_path))
    } else if (args.dossier) {
      argv.push('--dossier-json', '-')
      stdin = JSON.stringify(args.dossier)
    } else {
      argv.push('--inputs-json', '-')
      stdin = JSON.stringify(args.inputs ?? {})
    }
    if (cfg.requireDossier === true) argv.push('--require-dossier')
    if (args.keycard) argv.push('--keycard', String(args.keycard))
    if (args.backend) argv.push('--backend', String(args.backend))
  } else if (tool === 'intake') {
    argv = [...base, 'review']
    if (args.id) argv.push('--id', String(args.id))
    if (args.dossier_path) {
      argv.push('--dossier-file', String(args.dossier_path))
    } else {
      argv.push('--dossier-json', '-')
      stdin = JSON.stringify(args.dossier ?? {})
    }
  } else if (tool === 'spec') {
    argv = [...base, 'spec', String(args.id)]
    if (args.full === true) argv.push('--full')
  } else if (tool === 'catalogue') {
    argv = [...base, 'catalogue']
    if (args.query) argv.push('--query', String(args.query))
    if (args.kind) argv.push('--kind', String(args.kind))
    if (args.limit !== undefined) argv.push('--limit', String(args.limit))
    if (args.offset !== undefined) argv.push('--offset', String(args.offset))
  } else if (tool === 'version') {
    argv = [...base, 'version']
  } else {
    throw new Error(`azoth bridge: unknown tool ${tool}`)
  }
  return {
    command: argv[0],
    args: argv.slice(1),
    env,
    cwd: cfg.cwd,
    stdin,
  }
}

export function parseBridgeStdout(stdout) {
  const text = String(stdout ?? '').trim()
  if (!text) throw new Error('azoth bridge wrote no JSON')
  let document
  try {
    document = JSON.parse(text)
  } catch {
    throw new Error(`azoth bridge wrote non-JSON: ${text.slice(-500)}`)
  }
  if (document && document.kind === 'error') {
    throw refusal(document)
  }
  return document
}

export function runProcess(
  invocation,
  { timeoutMs = DEFAULT_TIMEOUT_MS, maxOutputBytes = DEFAULT_MAX_OUTPUT_BYTES, signal } = {},
) {
  return new Promise((resolve, reject) => {
    const child = spawn(invocation.command, invocation.args, {
      cwd: invocation.cwd,
      env: invocation.env,
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    let stdout = ''
    let stderr = ''
    let settled = false
    const finish = (callback) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (signal) signal.removeEventListener('abort', onAbort)
      callback()
    }
    const kill = () => {
      try {
        child.kill('SIGKILL')
      } catch {
        // The process has already left; the close handler is what settles this.
      }
    }
    const timer = setTimeout(() => {
      kill()
      finish(() =>
        reject(
          new Error(
            `The calculation took longer than ${Math.max(1, Math.round(timeoutMs / 1000))} s. ` +
              'Narrow the inputs or choose a simpler calculation.',
          ),
        ),
      )
    }, timeoutMs)
    const onAbort = () => {
      kill()
      finish(() => reject(new Error('azoth bridge aborted')))
    }
    if (signal) {
      if (signal.aborted) {
        onAbort()
        return
      }
      signal.addEventListener('abort', onAbort, { once: true })
    }
    child.on('error', (error) => finish(() => reject(error)))
    child.stdout.on('data', (chunk) => {
      stdout += chunk
      if (Buffer.byteLength(stdout) > maxOutputBytes) {
        kill()
        finish(() =>
          reject(
            new Error(
              'The result is too large to display. Ask for a smaller calculation or a ' +
                'narrower catalogue query.',
            ),
          ),
        )
      }
    })
    child.stderr.on('data', (chunk) => {
      stderr += chunk
      if (stderr.length > 65_536) stderr = stderr.slice(-65_536)
    })
    child.on('close', (code) => {
      finish(() => {
        if (code !== 0 && !stdout.trim()) {
          reject(
            new Error(
              `The calculation bridge failed (exit ${code}): ` +
                `${stderr.trim().slice(-500) || 'no detail'}`,
            ),
          )
          return
        }
        try {
          resolve(parseBridgeStdout(stdout))
        } catch (error) {
          reject(error)
        }
      })
    })
    if (invocation.stdin !== undefined) child.stdin.write(invocation.stdin)
    child.stdin.end()
  })
}

export async function runBridge(tool, args, cfg, exec = {}) {
  const invocation = buildInvocation(tool, args, cfg)
  return runProcess(invocation, {
    timeoutMs: cfg.timeoutMs ?? DEFAULT_TIMEOUT_MS,
    maxOutputBytes: cfg.maxOutputBytes ?? DEFAULT_MAX_OUTPUT_BYTES,
    signal: exec.signal,
  })
}

export async function selftest(cfg) {
  const version = await runBridge('version', {}, cfg)
  const catalogue = await runBridge('catalogue', { limit: 1 }, cfg)
  if (version.kind !== 'version' || catalogue.kind !== 'catalogue') {
    throw new Error('azoth plugin: the bridge answered an unexpected envelope')
  }
  console.log(`azoth plugin: ok (${version.calcs} calcs, ${version.models} models)`)
}
