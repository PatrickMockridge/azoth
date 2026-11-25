// The DSH plugin: three tools over `python -m azoth.harness`.
//
// It imports nothing from @deepseek-ai/*. `ToolsService.register` accepts a plain
// ToolDefinition (`output.schema`, `output.render`, `execute`), so the plugin mounts by
// absolute path without a node_modules tree and without the author-schema DSL that
// `defineTool` compiles. All engineering logic lives in the Python bridge; this file only
// validates the model's arguments and renders the envelope.

import { runBridge } from './bridge.js'

export const name = 'azoth-calc'
export const inject = ['tools']

const MAX_RENDER_BYTES = 65_536
const OUTPUT_SCHEMA = { type: 'object', additionalProperties: true }

function content(text) {
  const value = String(text)
  if (Buffer.byteLength(value) <= MAX_RENDER_BYTES) {
    return [{ type: 'text', text: value }]
  }
  return [
    {
      type: 'text',
      text: `${value.slice(0, MAX_RENDER_BYTES)}\n... truncated; narrow the query`,
    },
  ]
}

function renderJson(_args, value) {
  try {
    return content(JSON.stringify(value, null, 2))
  } catch {
    return content(String(value))
  }
}

function show(value) {
  if (typeof value === 'string') return JSON.stringify(value)
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (value === null || value === undefined) return 'null'
  return JSON.stringify(value)
}

function shortHash(value) {
  return typeof value === 'string' ? value.slice(0, 12) : 'unknown'
}

function renderResult(_args, value) {
  try {
    const result = value?.result ?? {}
    const lines = [`${value?.id ?? '?'} (${value?.backend ?? '?'})`]
    for (const [key, field] of Object.entries(result)) {
      if (key === 'provenance' || key === 'warnings') continue
      lines.push(`${key} = ${show(field)}`)
    }
    const warnings = Array.isArray(result.warnings) ? result.warnings : []
    const codes = warnings.map((warning) => warning?.code ?? String(warning))
    lines.push(`warnings: ${warnings.length}${codes.length ? ` (${codes.join(', ')})` : ''}`)
    const provenance = result.provenance ?? {}
    lines.push(
      `provenance: ${provenance.verification ?? 'unknown'}, ` +
        `spec ${shortHash(provenance.spec_sha256)}, rust ${shortHash(provenance.rust_sha256)}`,
    )
    return content(lines.join('\n'))
  } catch {
    return renderJson(_args, value)
  }
}

function tool({ name: toolName, description, parameters, execute, render = renderJson, timeoutMs }) {
  return {
    name: toolName,
    description,
    parameters,
    output: { schema: OUTPUT_SCHEMA, render },
    timeoutMs,
    async execute(args, exec) {
      return execute(args, exec)
    },
  }
}

function requireId(args) {
  if (typeof args?.id !== 'string' || args.id.length === 0) {
    throw new Error('azoth: `id` must be a non-empty string')
  }
}

function requireInputs(args) {
  if (args?.inputs === null || typeof args?.inputs !== 'object' || Array.isArray(args.inputs)) {
    throw new Error('azoth: `inputs` must be a JSON object')
  }
}

export function apply(ctx, config = {}) {
  if (!config.python) throw new Error('azoth-calc: config.python is required')
  const cfg = {
    python: config.python,
    pythonpath: config.pythonpath ?? [],
    cwd: config.cwd,
    timeoutMs: config.timeoutMs ?? 120_000,
    maxOutputBytes: config.maxOutputBytes ?? 4_194_304,
  }

  ctx.tools.register(
    tool({
      name: 'azoth_catalogue',
      description:
        'List azoth calculations and models by id. Use it to find an id before azoth_spec or azoth_calc.',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'case-insensitive id or name filter' },
          kind: { type: 'string', enum: ['calc', 'model'] },
          limit: { type: 'number', description: 'page size, 1..500' },
          offset: { type: 'number', description: 'page offset' },
        },
        additionalProperties: false,
      },
      timeoutMs: cfg.timeoutMs,
      execute: (args, exec) => runBridge('catalogue', args ?? {}, cfg, exec),
    }),
  )

  ctx.tools.register(
    tool({
      name: 'azoth_spec',
      description:
        'Read one azoth calculation or model declaration: inputs with their units, outputs, valid range, source and assumptions.',
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a registered id, e.g. hydraulics.darcy_weisbach' },
          full: { type: 'boolean', description: 'include the worked example and test cases' },
        },
        required: ['id'],
        additionalProperties: false,
      },
      timeoutMs: cfg.timeoutMs,
      execute: (args, exec) => {
        requireId(args)
        return runBridge('spec', args, cfg, exec)
      },
    }),
  )

  ctx.tools.register(
    tool({
      name: 'azoth_calc',
      description:
        'Run a registered azoth calculation or model by id. Dimensioned inputs are numbers in the spec unit or {value, unit}; the result carries warnings and provenance.',
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a registered id' },
          inputs: {
            type: 'object',
            additionalProperties: true,
            description: 'the spec inputs, in the units azoth_spec declares',
          },
          keycard: { type: 'string', description: 'a keycard path under the working directory' },
          backend: { type: 'string', enum: ['auto', 'python', 'rust'] },
        },
        required: ['id', 'inputs'],
        additionalProperties: false,
      },
      timeoutMs: cfg.timeoutMs,
      execute: (args, exec) => {
        requireId(args)
        requireInputs(args)
        return runBridge('run', args, cfg, exec)
      },
      render: renderResult,
    }),
  )
}
