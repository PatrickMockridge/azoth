// The argv, environment and JSON plumbing between a DSH tool and `python -m azoth.harness`.
//
// This file imports nothing from DeepSeek Harness, so `node --test` exercises it without a
// runtime, and the plugin that mounts it needs no node_modules.

import { spawn } from 'node:child_process'
import { delimiter } from 'node:path'

const DEFAULT_TIMEOUT_MS = 120_000
const DEFAULT_MAX_OUTPUT_BYTES = 4 * 1024 * 1024

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
    argv = [...base, 'run', String(args.id), '--inputs-json', '-']
    if (args.keycard) argv.push('--keycard', String(args.keycard))
    if (args.backend) argv.push('--backend', String(args.backend))
    stdin = JSON.stringify(args.inputs ?? {})
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
    const error = new Error(
      `${document.error?.type ?? 'Error'}: ${document.error?.message ?? 'unknown refusal'}`,
    )
    error.type = document.error?.type
    error.field = document.error?.field
    throw error
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
      finish(() => reject(new Error(`azoth bridge timed out after ${timeoutMs} ms`)))
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
          reject(new Error(`azoth bridge wrote more than ${maxOutputBytes} bytes`)),
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
              `azoth bridge exited ${code}: ${stderr.trim().slice(-500) || 'no stderr'}`,
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
