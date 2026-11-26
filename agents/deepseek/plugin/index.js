// The DSH plugin: five tools over `python -m azoth.harness`.
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

const HELP = {
  what:
    'azoth is a chemical-engineering calculation library: thermodynamics, hydraulics, ' +
    'heat transfer, unit operations and process safety. It is not a web project.',
  terms: {
    FEED: 'Front-End Engineering Design; not a browser front end.',
    model: 'a thermodynamic, property or equipment model; not an AI model.',
    catalogue: "azoth's list of calculations and models; use azoth_catalogue.",
    keycard: 'an optional data-entitlement card; omit it unless the user has one.',
    screening:
      'a value for study; not a design deliverable unless its verification says verified.',
    dossier:
      'a review of the supplied inputs: sources, claims, and how each input is bound.',
    'hard gap': 'an input that cannot be constructed from the evidence; the target is refused.',
    'soft gap': 'an input constructed by a licensed inference or a lower source rung; flagged.',
  },
  order: [
    'azoth_catalogue finds a calculation from plant words or an id.',
    'azoth_spec reads its inputs, units, valid range, assumptions and source.',
    'azoth_intake reviews the supplied evidence before a calculation runs.',
    'azoth_calc runs it and returns warnings, provenance and any intake block.',
  ],
  examples: [
    'Size a pump for 100 m³/h of water at 8 bar discharge.',
    'What is the pressure drop in 100 m of 0.1 m pipe at 1.5 m/s?',
    'Check the demo separator at its current conditions.',
    'What can azoth calculate for relief and fire cases?',
  ],
  flowsheet:
    'The flowsheet tools hold one document in memory and never write it. Export ' +
    'azoth://document and save the TOML if the user wants the edit kept.',
}

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

function verificationNote(value) {
  switch (value) {
    case 'verified':
      return 'externally checked against its validation cases'
    case 'partially_verified':
      return 'partly externally checked; screening value, not a design deliverable'
    case 'unverified':
      return 'not externally checked; screening value, not a design deliverable'
    case 'source_needed':
      return 'no external source found; do not use it as a design value'
    default:
      return 'verification not stated; treat it as screening'
  }
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
    const verification = provenance.verification ?? 'unknown'
    lines.push(`verification: ${verification} — ${verificationNote(verification)}`)
    lines.push(
      `provenance: spec ${shortHash(provenance.spec_sha256)}, ` +
        `rust ${shortHash(provenance.rust_sha256)}`,
    )
    if (value?.intake) {
      lines.push(
        '',
        `intake: ${value.intake.enforcement ?? 'advisory'} for ` +
          `${value.intake.target ?? value.id ?? '?'}`,
      )
      lines.push(...intakeLines(value.intake))
    }
    return content(lines.join('\n'))
  } catch {
    return renderJson(_args, value)
  }
}

function intakeLines(value) {
  const register = Array.isArray(value?.register) ? value.register : []
  const requests = Array.isArray(value?.requests) ? value.requests : []
  const hard = register.filter((gap) => gap.class === 'hard').length
  const soft = register.filter((gap) => gap.class === 'soft').length
  const lines = [`verdict: ${value?.verdict ?? 'unknown'} (${hard} hard, ${soft} soft)`]
  for (const gap of register) {
    lines.push(
      `  ${gap.gap} ${gap.class} ${gap.code} ${gap.input}: ${gap.detail}`,
    )
  }
  for (const request of requests) {
    lines.push(
      `  ${request.request}${request.blocking ? ' (blocking)' : ''} ${request.ask}`,
    )
  }
  return lines
}

function renderIntake(_args, value) {
  const dossier = value?.dossier
  const name = typeof dossier === 'string' ? dossier : (dossier?.id ?? '?')
  const lines = [`intake review: ${name}`, ...intakeLines(value)]
  return content(lines.join('\n'))
}

function renderHelp(_args, value) {
  try {
    const lines = [value?.what ?? 'azoth is a chemical-engineering calculation library.']
    lines.push('', 'Terms:')
    for (const [term, meaning] of Object.entries(value?.terms ?? {})) {
      lines.push(`${term}: ${meaning}`)
    }
    lines.push('', 'How to get a number:')
    for (const step of value?.order ?? []) lines.push(`- ${step}`)
    lines.push('', 'Example questions:')
    for (const example of value?.examples ?? []) lines.push(`- ${example}`)
    if (value?.flowsheet) lines.push('', value.flowsheet)
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
    throw new Error('azoth: `id` must be a calculation id from azoth_catalogue')
  }
}

function requireInputs(args) {
  if (args?.inputs === null || typeof args?.inputs !== 'object' || Array.isArray(args.inputs)) {
    throw new Error("azoth: `inputs` must be an object of the calculation's inputs")
  }
}

function dossierChoice(args) {
  const dossier = args?.dossier
  const path = args?.dossier_path
  if (dossier !== undefined && path !== undefined) {
    throw new Error('azoth: pass `dossier` or `dossier_path`, not both')
  }
  if (
    dossier !== undefined &&
    (dossier === null || typeof dossier !== 'object' || Array.isArray(dossier))
  ) {
    throw new Error('azoth: `dossier` must be an object')
  }
  if (path !== undefined && (typeof path !== 'string' || path.length === 0)) {
    throw new Error('azoth: `dossier_path` must be a path')
  }
  return { dossier, path }
}

export function apply(ctx, config = {}) {
  if (!config.python) throw new Error('azoth-calc: config.python is required')
  const cfg = {
    python: config.python,
    pythonpath: config.pythonpath ?? [],
    cwd: config.cwd,
    timeoutMs: config.timeoutMs ?? 120_000,
    maxOutputBytes: config.maxOutputBytes ?? 4_194_304,
    requireDossier: config.requireDossier === true,
  }

  ctx.tools.register(
    tool({
      name: 'azoth_help',
      description:
        'Read this first when the user asks what azoth can do, or uses FEED, model, ' +
        'catalogue, keycard or screening. It explains those terms and the order to call ' +
        'the calculation tools.',
      parameters: {
        type: 'object',
        properties: {},
        additionalProperties: false,
      },
      execute: () => HELP,
      render: renderHelp,
    }),
  )

  ctx.tools.register(
    tool({
      name: 'azoth_catalogue',
      description:
        'Find azoth calculations and fluid-property models from plant words or an id: ' +
        'pump power, pipe pressure drop, PSV sizing, separator capacity, hydrate. Returns ' +
        'ids and their input and output names. Call azoth_spec on a candidate before ' +
        'azoth_calc.',
      parameters: {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description: 'plant words or an id fragment, e.g. pump, PSV, hydrate',
          },
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
        "Read one calculation's engineering declaration: each input with its unit and " +
        'valid range, the outputs, assumptions, source and verification. Call this before ' +
        'azoth_calc so the inputs are in the units it asks for.',
      parameters: {
        type: 'object',
        properties: {
          id: {
            type: 'string',
            description: 'a calculation id from azoth_catalogue',
          },
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
      name: 'azoth_intake',
      description:
        'Review the supplied evidence before a calculation runs: the sources and claims, ' +
        'and how each input of a target is bound. Returns hard gaps (the target is ' +
        'refused), soft gaps (a licensed inference, a lower source rung) and the data ' +
        'requests. It does not calculate the target.',
      parameters: {
        type: 'object',
        properties: {
          dossier: {
            type: 'object',
            additionalProperties: true,
            description: 'the authored dossier: sources, claims and targets',
          },
          dossier_path: {
            type: 'string',
            description: 'a .toml or .json dossier under the working directory',
          },
          id: {
            type: 'string',
            description: 'review only this target id',
          },
        },
        additionalProperties: false,
      },
      timeoutMs: cfg.timeoutMs,
      execute: (args, exec) => {
        const { dossier, path } = dossierChoice(args ?? {})
        if (dossier === undefined && path === undefined) {
          throw new Error('azoth: `dossier` or `dossier_path` is required')
        }
        return runBridge('intake', args ?? {}, cfg, exec)
      },
      render: renderIntake,
    }),
  )

  ctx.tools.register(
    tool({
      name: 'azoth_calc',
      description:
        'Run an azoth calculation. `id` comes from azoth_catalogue and `inputs` are the ' +
        'values azoth_spec asks for, as numbers in its units or {value, unit} pairs. A ' +
        '`dossier` replaces `inputs` and adds an intake block to the result; a refused ' +
        'dossier must not be presented as a constructible basis. The result carries ' +
        'warnings and the source and verification of the number. Treat unverified results ' +
        'as screening, not design deliverables.',
      parameters: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a calculation id from azoth_catalogue' },
          inputs: {
            type: 'object',
            additionalProperties: true,
            description: "the calculation's inputs, in the units azoth_spec shows",
          },
          dossier: {
            type: 'object',
            additionalProperties: true,
            description: 'a reviewed dossier; it replaces `inputs`',
          },
          dossier_path: {
            type: 'string',
            description: 'a .toml or .json dossier under the working directory; replaces `inputs`',
          },
          keycard: {
            type: 'string',
            description:
              'optional path to a data-entitlement card under the working directory; ' +
              'omit unless the user has one',
          },
          backend: {
            type: 'string',
            enum: ['auto', 'python', 'rust'],
            description:
              'leave auto unless the user asks to compare the Python and Rust implementations',
          },
        },
        required: ['id'],
        additionalProperties: false,
      },
      timeoutMs: cfg.timeoutMs,
      execute: (args, exec) => {
        requireId(args)
        const { dossier, path } = dossierChoice(args)
        const hasDossier = dossier !== undefined || path !== undefined
        if (hasDossier && args.inputs !== undefined) {
          throw new Error('azoth: a dossier replaces `inputs`; pass one or the other')
        }
        if (!hasDossier) {
          if (cfg.requireDossier) {
            throw new Error(
              'azoth: this session requires an intake dossier; call azoth_intake first',
            )
          }
          requireInputs(args)
        }
        return runBridge('run', args, cfg, exec)
      },
      render: renderResult,
    }),
  )
}
