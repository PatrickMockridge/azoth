// The DSH planning guard: a decision tree over azoth calculations.
//
// It imports nothing from @deepseek-ai/*. `ctx.tools.guard` holds `azoth_calc` while a
// decision is tripped or a review is due; `tools/post-execute` prepends the reason to the
// result; `ctx.systemPrompt` carries the protocol. State is per session in this process,
// and every plan result and every trip or checkpoint banner carries a bounded plan-record
// line for the human and for resume.

export const name = 'azoth-plan'
export const inject = ['tools']

const CATEGORIES = [
  'numerical',
  'validity',
  'evidence',
  'applicability',
  'consistency',
  'feasibility',
  'requirement',
  'budget',
  'judgment',
]
const FALSIFIERS = [
  'error',
  'non_finite',
  'hard_warning',
  'any_warning',
  'out_of_range',
  'intake_refused',
  'intake_hard_gap',
  'manual',
]
const AUTOMATIC = new Set(FALSIFIERS.filter((falsifier) => falsifier !== 'manual'))
const DEFAULT_HARD = ['SOLVER_NOT_CONVERGED', 'TRIVIAL_SOLUTION', 'FALLBACK_TO_INPUT']
const DEFAULT_CATEGORIES = {
  SOLVER_NOT_CONVERGED: 'numerical',
  TRIVIAL_SOLUTION: 'numerical',
  FALLBACK_TO_INPUT: 'numerical',
  OUT_OF_VALID_RANGE: 'validity',
  RANGE_CHECK_SKIPPED: 'validity',
  TRANSITIONAL_FLOW: 'validity',
  MISSING_EVIDENCE: 'evidence',
}
const DEFAULT_POLICY =
  'Open a plan before multi-step work. Record candidates, assumptions and the reason a ' +
  'result changes the decision. A trip holds calculations until the decision is reopened ' +
  'or the assumption is resolved with evidence.'
const SNAPSHOT_MARKER = 'plan-record (guard state; do not edit):'
const MAX_RENDER_BYTES = 65_536
const MAX_CANDIDATES = 12
const MAX_ASSUMPTIONS = 20
const MAX_TOTAL_ASSUMPTIONS = 100
const MAX_DECISIONS = 20
const MAX_STEPS = 100
const MAX_TRIPS = 200
const MAX_PERMITS = 20
const MAX_EXTERNAL_LOG = 60
const MAX_QUESTIONS = 12
const MAX_BASIS = 40
const MAX_FAILURES = 100
const MAX_TARGETS = 10
const MAX_COMPARISONS = 50
const MAX_JOBS = 30
const PERMIT_SCOPES = ['once', 'step', 'session']
const STEP_KINDS = [
  'lookup',
  'thermo',
  'steady',
  'transport',
  'transient',
  'field',
  'solver',
  'check',
  'report',
]
const STEP_STATUSES = ['active', 'done', 'failed']
const FIDELITIES = ['thermo', 'steady', 'transient', 'field', 'olga']
const PRIORITIES = ['primary', 'secondary']
const BASIS_CHECKS = [
  'flow',
  'diameter',
  'pressure_drop',
  'composition',
  'units',
  'gauge',
  'consistency',
]
const BASIS_VERDICTS = ['pass', 'warn', 'fail']
const REQUIRED_BASIS = {
  thermo: ['composition', 'consistency'],
  steady: ['composition', 'consistency'],
  transport: ['flow', 'diameter', 'pressure_drop', 'composition', 'consistency'],
  transient: ['flow', 'diameter', 'pressure_drop', 'composition', 'consistency'],
  field: ['flow', 'diameter', 'pressure_drop', 'composition', 'consistency'],
  solver: ['flow', 'diameter', 'pressure_drop', 'composition', 'consistency'],
  olga: ['flow', 'diameter', 'pressure_drop', 'composition', 'consistency'],
}

const OUTPUT_SCHEMA = { type: 'object', additionalProperties: true }

let sequence = 0
const plans = new Map()
//: exec -> {permitId, command, mode} for a bash call the pre-execute stage let through.
const externalRuns = new WeakMap()

function nextSequence() {
  sequence += 1
  return sequence
}

function now() {
  return new Date().toISOString()
}

function uuid() {
  try {
    return globalThis.crypto?.randomUUID?.() ?? `azoth-plan-${nextSequence()}`
  } catch {
    return `azoth-plan-${nextSequence()}`
  }
}

function content(text) {
  const value = String(text ?? '')
  if (Buffer.byteLength(value) <= MAX_RENDER_BYTES) return [{ type: 'text', text: value }]
  return [
    {
      type: 'text',
      text: `${value.slice(0, MAX_RENDER_BYTES)}\n... truncated; call azoth_plan status for the record`,
    },
  ]
}

function has(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key)
}

function normalizeConfig(config = {}) {
  const hard = Array.isArray(config.hardWarningCodes)
    ? config.hardWarningCodes
    : DEFAULT_HARD
  const categories = {}
  for (const [code, category] of Object.entries(DEFAULT_CATEGORIES)) {
    categories[code] = category
  }
  for (const [code, category] of Object.entries(config.warningCategories ?? {})) {
    if (typeof category === 'string' && CATEGORIES.includes(category)) {
      categories[code] = category
    }
  }
  return {
    policy: typeof config.policy === 'string' && config.policy.trim() ? config.policy : DEFAULT_POLICY,
    requirePlanAfter: integer(config.requirePlanAfter, 1, 0, 1000),
    minimumCandidates: integer(config.minimumCandidates, 2, 2, MAX_CANDIDATES),
    reviewAfterCalcs: integer(config.reviewAfterCalcs, 6, 0, 1000),
    hardCodes: new Set(hard.filter((code) => typeof code === 'string')),
    categories,
    resolveRequiresCleanCalc: config.resolveRequiresCleanCalc === true,
    externalCompute: ['ask', 'deny', 'allow'].includes(config.externalCompute)
      ? config.externalCompute
      : 'ask',
    gateSanctionedBridge: config.gateSanctionedBridge !== false,
    reviewAfterMinutes: integer(config.reviewAfterMinutes, 30, 0, 10_080),
    failureTripAfter: integer(config.failureTripAfter, 3, 0, 100),
  }
}

function integer(value, fallback, minimum, maximum) {
  if (!Number.isInteger(value)) return fallback
  return Math.min(maximum, Math.max(minimum, value))
}

// ---------------------------------------------------------------------------------------
// bash classification
//
// The guard sees the shell command, not what it computes. These classes are the policy
// boundary: azoth's own tools pass, read-only inspection passes, and an interpreter is
// external work that needs a permit.

const READONLY_COMMANDS = new Set([
  'cat', 'head', 'tail', 'less', 'more', 'wc', 'ls', 'stat', 'file', 'pwd', 'tree', 'grep',
  'rg', 'diff', 'sort', 'uniq', 'cut', 'tr', 'column', 'jq', 'sha256sum', 'md5sum', 'date',
  'which', 'echo', 'printf', 'true', 'false', 'test', 'basename', 'dirname', 'realpath', 'du',
  'df', 'ps', 'nproc', 'free',
])
const NEUTRAL_COMMANDS = new Set([
  'mkdir', 'rmdir', 'cp', 'mv', 'rm', 'touch', 'chmod', 'chown', 'ln', 'sed', 'sleep', 'kill',
  'export', 'unset', 'cd', 'set', 'wait', 'jobs',
])
const GIT_READ = new Set([
  'status', 'log', 'diff', 'show', 'branch', 'rev-parse', 'ls-files', 'grep', 'blame',
  'describe', 'remote', 'tag',
])
const EXTERNAL_COMMANDS = new Set([
  'python', 'python3', 'pytest', 'maturin', 'cargo', 'rustc', 'node', 'npm', 'npx', 'yarn',
  'pnpm', 'uv', 'pip', 'Rscript', 'julia', 'java', 'javac', 'make', 'cmake', 'ninja', 'bash',
  'sh', 'zsh', 'perl', 'ruby', 'php', 'go', 'dotnet', 'gcc', 'g++', 'clang', 'ffmpeg',
  'octave', 'sqlite3', 'psql', 'mysql', 'awk', 'xargs', 'source', '.',
])
const CLASS_RANK = {
  readonly: 0,
  'sanctioned-read': 1,
  neutral: 2,
  'sanctioned-compute': 3,
  external: 4,
}

//: A heredoc body is data, not a command line: drop it before splitting, or `cat > f <<EOF`
//: over a Python fragment reads as an interpreter run.
function withoutHeredocBodies(command) {
  const lines = command.split('\n')
  const kept = []
  for (let index = 0; index < lines.length; index += 1) {
    kept.push(lines[index])
    const marker = lines[index].match(/<<-?\s*['"]?([A-Za-z_][A-Za-z0-9_]*)['"]?/)
    if (!marker) continue
    const tag = marker[1]
    while (index + 1 < lines.length) {
      index += 1
      if (lines[index].trim() === tag) break
    }
  }
  return kept.join('\n')
}

function stripCommandPrefix(segment) {
  let rest = segment.trim()
  let changed = true
  while (changed && rest) {
    changed = false
    const assignment = rest.match(/^[A-Za-z_][A-Za-z0-9_]*=\S*\s+/)
    if (assignment) {
      rest = rest.slice(assignment[0].length)
      changed = true
      continue
    }
    const first = rest.split(/\s+/, 1)[0]
    if (['sudo', 'env', 'nohup', 'time', 'nice', 'command', 'exec'].includes(first)) {
      rest = rest.slice(first.length).trim()
      changed = true
      continue
    }
    if (first === 'timeout') {
      const duration = rest.match(/^timeout\s+(?:-\S+\s+)*\d+(?:\.\d+)?[smhd]?\s+/)
      if (duration) {
        rest = rest.slice(duration[0].length)
        changed = true
      }
    }
  }
  return rest
}

function classifySegment(segment) {
  const text = stripCommandPrefix(segment)
  if (!text) return 'readonly'
  const tokens = text.split(/\s+/)
  const head = tokens[0].toLowerCase()
  const args = tokens.slice(1)
  const subcommand = args.find((arg) => !arg.startsWith('-'))?.toLowerCase()
  if (head === 'azoth') {
    if (['catalogue', 'spec', 'version', 'help'].includes(subcommand)) return 'sanctioned-read'
    if (['run', 'review'].includes(subcommand)) return 'sanctioned-compute'
    return 'external'
  }
  if (head === 'python' || head === 'python3') {
    const module = args.indexOf('-m')
    if (module >= 0 && args[module + 1] === 'azoth.harness') {
      const sub = args.slice(module + 2).find((arg) => !arg.startsWith('-'))?.toLowerCase()
      if (['catalogue', 'spec', 'version', 'help'].includes(sub)) return 'sanctioned-read'
      if (['run', 'review'].includes(sub)) return 'sanctioned-compute'
    }
    return 'external'
  }
  if (head === 'find') {
    if (args.some((arg) => ['-exec', '-execdir', '-delete'].includes(arg))) return 'external'
    return 'readonly'
  }
  if (head === 'git') return GIT_READ.has(subcommand) ? 'readonly' : 'neutral'
  if (READONLY_COMMANDS.has(head)) return 'readonly'
  if (NEUTRAL_COMMANDS.has(head)) return 'neutral'
  if (EXTERNAL_COMMANDS.has(head)) return 'external'
  return 'external'
}

function classifyCommand(command) {
  const text = String(command ?? '')
  if (!text.trim()) return 'readonly'
  if (/\$\(|`/.test(text)) return 'external'
  let best = 'readonly'
  for (const segment of withoutHeredocBodies(text).split(/&&|\|\||[;|&\n]/)) {
    const kind = classifySegment(segment)
    if (CLASS_RANK[kind] > CLASS_RANK[best]) best = kind
  }
  return best
}

// ---------------------------------------------------------------------------------------
// state

function createState() {
  return {
    v: 1,
    problem: '',
    criteria: [],
    decisions: [],
    steps: [],
    trips: [],
    reviews: [],
    permits: [],
    externalLog: [],
    questions: [],
    basis: [],
    failures: [],
    targets: [],
    comparisons: [],
    jobs: [],
    activeStepId: null,
    ceremony: 'simple',
    reviewWindowStartedAt: Date.now(),
    calcCount: 0,
    calcSinceReview: 0,
    calcLog: [],
    createdAt: now(),
    updatedAt: now(),
  }
}

function createDecision(id, parent, question, candidates) {
  return {
    id,
    parent,
    question,
    candidates,
    choice: null,
    revisions: [],
    status: 'open',
  }
}

function findDecision(state, id) {
  return state.decisions.find((decision) => decision.id === id)
}

function findAssumption(state, id) {
  for (const decision of state.decisions) {
    for (const candidate of decision.candidates) {
      const assumption = candidate.assumptions.find((item) => item.id === id)
      if (assumption) return { decision, candidate, assumption }
    }
  }
  return null
}

function assumptionIds(state) {
  const ids = new Set()
  for (const decision of state.decisions) {
    for (const candidate of decision.candidates) {
      for (const assumption of candidate.assumptions) ids.add(assumption.id)
    }
  }
  return ids
}

function candidateIds(state) {
  const ids = new Set()
  for (const decision of state.decisions) {
    for (const candidate of decision.candidates) ids.add(candidate.id)
  }
  return ids
}

function nextId(items, prefix) {
  let maximum = 0
  for (const item of items) {
    const match = new RegExp(`^${prefix}(\\d+)$`).exec(String(item.id ?? ''))
    if (match) maximum = Math.max(maximum, Number(match[1]))
  }
  return `${prefix}${maximum + 1}`
}

function openTrips(state, includeNotes = false) {
  return state.trips.filter(
    (trip) =>
      (includeNotes || trip.severity === 'trip') &&
      (trip.status === 'open' || trip.status === 'reopened'),
  )
}

function tripsFor(state, decisionId) {
  return openTrips(state).filter((trip) => trip.decision === decisionId)
}

function chosenCandidate(decision) {
  if (!decision?.choice) return null
  return decision.candidates.find((candidate) => candidate.id === decision.choice.candidate) ?? null
}

// A reopened trip is history: it marks the assumption it names, but it does not hold a
// new choice. Only an open trip on the chosen candidate (or a decision-level trip) holds.
function holdingTrips(state) {
  const found = []
  for (const decision of state.decisions) {
    if (!decision.choice) continue
    const candidate = chosenCandidate(decision)
    for (const trip of state.trips) {
      if (trip.severity !== 'trip' || trip.status !== 'open') continue
      if (trip.decision !== decision.id) continue
      if (trip.assumption === null) {
        found.push(trip)
        continue
      }
      if (candidate?.assumptions.some((assumption) => assumption.id === trip.assumption)) {
        found.push(trip)
      }
    }
  }
  return found
}

function assumptionStatus(state, id) {
  const trips = state.trips.filter(
    (trip) => trip.assumption === id && trip.severity === 'trip',
  )
  if (trips.some((trip) => trip.status === 'open' || trip.status === 'reopened')) {
    return 'falsified'
  }
  if (trips.length) return 'resolved'
  return 'open'
}

function ancestorIds(state, decision) {
  const ids = []
  let current = decision
  while (current?.parent) {
    ids.push(current.parent)
    current = findDecision(state, current.parent)
  }
  return ids
}

function descendants(state, id) {
  const found = []
  const queue = [id]
  while (queue.length) {
    const parent = queue.shift()
    for (const decision of state.decisions) {
      if (decision.parent === parent && !found.includes(decision)) {
        found.push(decision)
        queue.push(decision.id)
      }
    }
  }
  return found
}

function decisionState(state, decision) {
  if (!decision) return 'empty'
  if (decision.status === 'closed') return 'closed'
  if (decision.status === 'stale') return 'stale'
  if (decision.parent) {
    const parent = findDecision(state, decision.parent)
    if (parent && ['stale', 'closed'].includes(decisionState(state, parent))) return 'stale'
  }
  if (decision.choice && holdingTrips(state).some((trip) => trip.decision === decision.id)) {
    return 'tripped'
  }
  if (decision.choice) return 'chosen'
  return 'open'
}

function activeDecision(state) {
  let current = state.decisions.find((decision) => decision.parent === null)
  let active = null
  const seen = new Set()
  while (current && !seen.has(current.id)) {
    seen.add(current.id)
    const status = decisionState(state, current)
    if (status !== 'chosen') break
    active = current
    const children = state.decisions.filter(
      (decision) => decision.parent === current.id && decisionState(state, decision) === 'chosen',
    )
    current = children[children.length - 1]
  }
  return active
}

function stepStale(state, step) {
  if (step.stale) return true
  for (const id of step.depends_on ?? []) {
    const decision = findDecision(state, id)
    if (decision) {
      if (['stale', 'closed'].includes(decisionState(state, decision))) return true
      continue
    }
    const found = findAssumption(state, id)
    if (found && assumptionStatus(state, id) === 'falsified') return true
  }
  return false
}

// ---------------------------------------------------------------------------------------
// validation

function text(value, field, limit) {
  if (typeof value !== 'string' || !value.trim()) {
    throw new Error(`azoth_plan: \`${field}\` must be a non-empty string`)
  }
  return value.trim().slice(0, limit)
}

function optionalText(value, field, limit) {
  if (value === undefined || value === null) return null
  return text(value, field, limit)
}

function stringList(value, field, minimum, maximum, limit) {
  if (!Array.isArray(value)) {
    throw new Error(`azoth_plan: \`${field}\` must be an array of strings`)
  }
  if (value.length < minimum || value.length > maximum) {
    throw new Error(`azoth_plan: \`${field}\` must have ${minimum}..${maximum} entries`)
  }
  return value.map((item, index) => text(item, `${field}[${index}]`, limit))
}

function category(value) {
  if (typeof value !== 'string' || !CATEGORIES.includes(value)) {
    throw new Error(`azoth_plan: \`category\` must be one of ${CATEGORIES.join(', ')}`)
  }
  return value
}

function parseEstimate(value, field) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`azoth_plan: \`${field}\` must be an object {minutes, compute, risk}`)
  }
  const minutes = Number(value.minutes)
  if (!Number.isFinite(minutes) || minutes <= 0) {
    throw new Error(`azoth_plan: \`${field}.minutes\` must be a positive number`)
  }
  return {
    minutes,
    compute: text(value.compute, `${field}.compute`, 300),
    risk: text(value.risk, `${field}.risk`, 300),
  }
}

function parseQuestions(state, raw, decisionId, candidates) {
  if (raw === undefined) return []
  if (!Array.isArray(raw)) throw new Error('azoth_plan: `questions` must be an array')
  if (raw.length > MAX_QUESTIONS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_QUESTIONS} questions`)
  }
  const candidateIds = new Set(candidates.map((candidate) => candidate.id))
  let serial = Number(nextId(state.questions, 'Q').slice(1)) - 1
  return raw.map((item, index) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new Error(`azoth_plan: questions[${index}] must be an object`)
    }
    serial += 1
    const id = `Q${serial}`
    const body = text(item.text, `questions[${index}].text`, 500)
    if (!FIDELITIES.includes(item.fidelity)) {
      throw new Error(
        `azoth_plan: question ${id} needs fidelity (${FIDELITIES.join(', ')})`,
      )
    }
    const priority = item.priority === undefined ? 'primary' : item.priority
    if (!PRIORITIES.includes(priority)) {
      throw new Error(`azoth_plan: question ${id} needs priority (${PRIORITIES.join(', ')})`)
    }
    const answerBy = text(item.answer_by, `question ${id}.answer_by`, 80)
    if (!candidateIds.has(answerBy)) {
      throw new Error(`azoth_plan: question ${id} names unknown candidate "${answerBy}"`)
    }
    return {
      id,
      decision: decisionId,
      text: body,
      fidelity: item.fidelity,
      priority,
      answer_by: answerBy,
    }
  })
}

function attachQuestionAnswers(candidates, questions) {
  for (const question of questions) {
    const candidate = candidates.find((item) => item.id === question.answer_by)
    if (candidate && !candidate.answers.includes(question.id)) {
      candidate.answers.push(question.id)
    }
  }
}

function requireStudyFields(state, candidates, questions) {
  if (!questions.length && state.ceremony !== 'study') return
  if (questions.length) state.ceremony = 'study'
  const all = state.decisions.flatMap((decision) => decision.candidates).concat(candidates)
  for (const candidate of all) {
    if (!candidate.estimate) {
      throw new Error(
        `azoth_plan: study candidate "${candidate.id}" needs estimate {minutes, compute, risk}`,
      )
    }
    if (!candidate.fallback) {
      throw new Error(`azoth_plan: study candidate "${candidate.id}" needs a fallback`)
    }
    if (!candidate.fidelities.length) {
      throw new Error(
        `azoth_plan: study candidate "${candidate.id}" needs at least one fidelity ` +
          `(${FIDELITIES.join(', ')})`,
      )
    }
  }
}

function validateCandidates(
  state,
  raw,
  cfg,
  existingAssumptions,
  seedCandidates = candidateIds(state),
) {
  if (!Array.isArray(raw)) throw new Error('azoth_plan: `candidates` must be an array')
  if (raw.length < cfg.minimumCandidates || raw.length > MAX_CANDIDATES) {
    throw new Error(
      `azoth_plan: concept selection needs ${cfg.minimumCandidates}..${MAX_CANDIDATES} candidates`,
    )
  }
  const seenCandidates = new Set(seedCandidates)
  const seenAssumptions = new Set(existingAssumptions)
  const candidates = []
  let automatic = false
  let total = existingAssumptions.size
  for (const item of raw) {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new Error('azoth_plan: each candidate must be an object')
    }
    const id = text(item.id, 'candidate.id', 80)
    if (seenCandidates.has(id)) throw new Error(`azoth_plan: duplicate candidate id "${id}"`)
    seenCandidates.add(id)
    const path = text(item.path, `candidate ${id}.path`, 2000)
    const cost = text(item.cost, `candidate ${id}.cost`, 500)
    const estimate =
      item.estimate === undefined ? null : parseEstimate(item.estimate, `candidate ${id}.estimate`)
    const fallback =
      item.fallback === undefined ? null : text(item.fallback, `candidate ${id}.fallback`, 500)
    const answers =
      item.answers === undefined
        ? []
        : stringList(item.answers, `candidate ${id}.answers`, 0, MAX_QUESTIONS, 40)
    const fidelities =
      item.fidelities === undefined
        ? []
        : stringList(item.fidelities, `candidate ${id}.fidelities`, 0, FIDELITIES.length, 40)
    for (const fidelity of fidelities) {
      if (!FIDELITIES.includes(fidelity)) {
        throw new Error(
          `azoth_plan: candidate "${id}" has unknown fidelity "${fidelity}" ` +
            `(${FIDELITIES.join(', ')})`,
        )
      }
    }
    if (!Array.isArray(item.assumptions) || item.assumptions.length === 0) {
      throw new Error(`azoth_plan: candidate "${id}" needs at least one assumption`)
    }
    if (item.assumptions.length > MAX_ASSUMPTIONS) {
      throw new Error(`azoth_plan: candidate "${id}" has more than ${MAX_ASSUMPTIONS} assumptions`)
    }
    const assumptions = []
    for (const entry of item.assumptions) {
      if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        throw new Error(`azoth_plan: each assumption of "${id}" must be an object`)
      }
      const assumptionId = text(entry.id, `candidate ${id}.assumption.id`, 80)
      if (seenAssumptions.has(assumptionId)) {
        throw new Error(`azoth_plan: duplicate assumption id "${assumptionId}"`)
      }
      seenAssumptions.add(assumptionId)
      total += 1
      if (total > MAX_TOTAL_ASSUMPTIONS) {
        throw new Error(`azoth_plan: a plan is limited to ${MAX_TOTAL_ASSUMPTIONS} assumptions`)
      }
      const body = text(entry.text, `assumption ${assumptionId}.text`, 2000)
      if (typeof entry.category !== 'string' || !CATEGORIES.includes(entry.category)) {
        throw new Error(
          `azoth_plan: assumption "${assumptionId}" needs category (${CATEGORIES.join(', ')})`,
        )
      }
      const falsifier = entry.falsifier
      if (typeof falsifier !== 'string' || !FALSIFIERS.includes(falsifier)) {
        throw new Error(
          `azoth_plan: assumption "${assumptionId}" needs falsifier (${FALSIFIERS.join(', ')})`,
        )
      }
      let calc = null
      if (falsifier !== 'manual') {
        calc = text(entry.calc, `assumption ${assumptionId}.calc`, 120)
        if (falsifier.startsWith('intake_') && calc !== 'intake') {
          throw new Error(
            `azoth_plan: assumption "${assumptionId}" is an intake falsifier; set calc to "intake"`,
          )
        }
        if (!falsifier.startsWith('intake_') && calc === 'intake') {
          throw new Error(
            `azoth_plan: assumption "${assumptionId}" binds intake but uses a calculation falsifier`,
          )
        }
        automatic = true
      }
      assumptions.push({
        id: assumptionId,
        text: body,
        category: entry.category,
        falsifier,
        ...(calc ? { calc } : {}),
      })
    }
    candidates.push({
      id,
      path,
      cost,
      assumptions,
      ...(estimate ? { estimate } : {}),
      ...(fallback ? { fallback } : {}),
      answers,
      fidelities,
    })
  }
  if (!automatic) {
    throw new Error(
      'azoth_plan: at least one assumption must bind a calculation or intake to an automatic falsifier',
    )
  }
  return candidates
}

// ---------------------------------------------------------------------------------------
// trips

function addTrip(state, trip) {
  const duplicate = state.trips.find(
    (item) =>
      item.decision === trip.decision &&
      item.assumption === (trip.assumption ?? null) &&
      item.signal === trip.signal &&
      item.category === trip.category &&
      (item.status === 'open' || item.status === 'reopened'),
  )
  if (duplicate) {
    duplicate.evidence = trip.evidence ?? duplicate.evidence
    duplicate.calcId = trip.calcId ?? duplicate.calcId
    duplicate.callId = trip.callId ?? duplicate.callId
    duplicate.at = now()
    duplicate.severity = trip.severity ?? duplicate.severity
    return duplicate
  }
  const record = {
    id: nextId(state.trips, 'T'),
    decision: trip.decision,
    assumption: trip.assumption ?? null,
    text: trip.text ?? '',
    category: trip.category,
    signal: trip.signal,
    evidence: trip.evidence ?? '',
    severity: trip.severity ?? 'trip',
    automatic: trip.automatic === true,
    calcId: trip.calcId ?? null,
    callId: trip.callId ?? null,
    codes: Array.isArray(trip.codes) ? trip.codes : [],
    detail: trip.detail ?? {},
    status: 'open',
    calcSeq: state.calcCount,
    at: now(),
    resolvedBy: null,
  }
  state.trips.push(record)
  if (state.trips.length > MAX_TRIPS) {
    const index = state.trips.findIndex((item) => item.status === 'resolved')
    state.trips.splice(index >= 0 ? index : 0, 1)
  }
  state.updatedAt = now()
  return record
}

function falsifierMatches(falsifier, signal) {
  if (!falsifier || falsifier === 'manual') return false
  if (signal.kind === 'error') return falsifier === 'error' || falsifier === 'hard_warning'
  if (signal.kind === 'non_finite') return falsifier === 'non_finite' || falsifier === 'hard_warning'
  if (signal.kind === 'converged_false') return falsifier === 'hard_warning'
  if (signal.kind === 'intake') {
    return falsifier === 'intake_refused' || falsifier === 'intake_hard_gap' || falsifier === 'any_warning'
  }
  if (signal.kind === 'code') {
    if (signal.hard) return falsifier === 'hard_warning' || falsifier === 'any_warning'
    if (falsifier === 'any_warning') return true
    if (falsifier === 'out_of_range') return signal.signal === 'OUT_OF_VALID_RANGE'
    return false
  }
  return false
}

function bound(assumption, signal, calcId) {
  if (signal.kind === 'intake') return assumption.calc === 'intake'
  if (assumption.calc === '*') return true
  return assumption.calc === calcId
}

function applySignals(state, signals, options) {
  if (!state || !signals.length) return { trips: [], falsified: [] }
  const { calcId, callId, cfg } = options
  const created = []
  const falsified = new Set()
  const openDecision = state.decisions.find((decision) => decisionState(state, decision) === 'open')
  const active = activeDecision(state)
  for (const signal of signals) {
    let matched = false
    for (const decision of state.decisions) {
      if (decision.status === 'closed') continue
      for (const candidate of decision.candidates) {
        for (const assumption of candidate.assumptions) {
          if (!bound(assumption, signal, calcId)) continue
          if (!falsifierMatches(assumption.falsifier, signal)) continue
          matched = true
          falsified.add(assumption.id)
          const trip = addTrip(state, {
            decision: decision.id,
            assumption: assumption.id,
            text: assumption.text,
            category: assumption.category,
            signal: signal.signal,
            evidence: signal.evidence ?? '',
            severity: 'trip',
            automatic: true,
            calcId,
            callId,
            codes: signal.codes ?? [],
            detail: signal.detail ?? {},
          })
          if (!created.includes(trip)) created.push(trip)
        }
      }
    }
    if (!matched && signal.hard && !openDecision && active) {
      const trip = addTrip(state, {
        decision: active.id,
        assumption: null,
        text: `the chosen path in ${active.id}`,
        category: signal.category ?? 'numerical',
        signal: signal.signal,
        evidence: signal.evidence ?? '',
        severity: 'trip',
        automatic: true,
        calcId,
        callId,
        codes: signal.codes ?? [],
        detail: signal.detail ?? {},
      })
      if (!created.includes(trip)) created.push(trip)
    }
  }
  return { trips: created, falsified: [...falsified] }
}

function evaluateCalc(result, cfg) {
  if (!result || result.isError) {
    return [
      {
        kind: 'error',
        signal: 'error',
        category: 'numerical',
        hard: true,
        evidence: result?.error?.message ? String(result.error.message).slice(0, 500) : 'the calculation failed',
        detail: { type: result?.error?.type ?? 'error' },
      },
    ]
  }
  const document = result.value ?? {}
  const output = document.result ?? {}
  const codes = new Set()
  const provenance = output.provenance ?? {}
  if (Array.isArray(provenance.warning_codes)) {
    for (const code of provenance.warning_codes) if (typeof code === 'string') codes.add(code)
  }
  if (Array.isArray(output.warnings)) {
    for (const warning of output.warnings) {
      if (typeof warning?.code === 'string') codes.add(warning.code)
    }
  }
  const signals = []
  let nonFinite = false
  let convergedFalse = false
  const detail = {}
  walk(
    output,
    (node) => {
      if (
        has(node, 'magnitude') &&
        node.magnitude === null &&
        typeof node.unit === 'string' &&
        node.skipped !== true
      ) {
        nonFinite = true
      }
      if (has(node, 'converged') && node.converged === false) convergedFalse = true
      for (const key of ['converged', 'residual', 'iterations', 'skipped']) {
        if (detail[key] === undefined && has(node, key)) detail[key] = node[key]
      }
    },
    { depth: 0, nodes: 20_000 },
  )
  if (nonFinite) {
    signals.push({
      kind: 'non_finite',
      signal: 'non_finite',
      category: 'numerical',
      hard: true,
      evidence: 'a quantity serialised as a non-finite value',
      detail,
    })
  }
  if (convergedFalse) {
    signals.push({
      kind: 'converged_false',
      signal: 'converged_false',
      category: 'numerical',
      hard: true,
      evidence: 'the solver reported converged: false',
      detail,
    })
  }
  for (const code of codes) {
    const hard = cfg.hardCodes.has(code)
    signals.push({
      kind: 'code',
      signal: code,
      category: cfg.categories[code] ?? (hard ? 'numerical' : 'validity'),
      hard,
      evidence: `warning ${code}`,
      codes: [code],
      detail,
    })
  }
  const intake = document.intake
  if (intake) {
    const verdict = intake.verdict
    const hardGap =
      Array.isArray(intake.register) && intake.register.some((gap) => gap?.class === 'hard')
    if (verdict === 'refused' || hardGap) {
      signals.push({
        kind: 'intake',
        signal: verdict === 'refused' ? 'intake_refused' : 'intake_hard_gap',
        category: 'evidence',
        hard: true,
        evidence: `intake ${verdict ?? 'flagged'} for ${document.id ?? intake.target ?? 'the target'}`,
        codes: [],
        detail: {},
      })
    }
  }
  return signals
}

function evaluateIntake(result) {
  if (!result || result.isError) return []
  const value = result.value ?? {}
  const verdict = value.verdict ?? value.intake?.verdict
  const register = value.register ?? value.intake?.register ?? []
  const hardGap = Array.isArray(register) && register.some((gap) => gap?.class === 'hard')
  if (verdict !== 'refused' && !hardGap) return []
  return [
    {
      kind: 'intake',
      signal: verdict === 'refused' ? 'intake_refused' : 'intake_hard_gap',
      category: 'evidence',
      hard: true,
      evidence: `intake ${verdict ?? 'flagged'}${hardGap ? ' with a hard gap' : ''}`,
      codes: [],
      detail: {},
    },
  ]
}

function walk(value, visit, state) {
  if (state.nodes <= 0 || state.depth > 32) return
  state.nodes -= 1
  if (value === null || typeof value !== 'object') return
  if (Array.isArray(value)) {
    for (const item of value) {
      state.depth += 1
      walk(item, visit, state)
      state.depth -= 1
    }
    return
  }
  visit(value)
  for (const child of Object.values(value)) {
    state.depth += 1
    walk(child, visit, state)
    state.depth -= 1
  }
}

// ---------------------------------------------------------------------------------------
// holds

function holdReason(state, cfg) {
  if (!state) return undefined
  const active = state.decisions.filter((decision) => decision.status !== 'closed')
  if (!active.length) {
    if (state.calcCount >= cfg.requirePlanAfter) {
      return {
        reason:
          `azoth_calc is held: ${state.calcCount} calculation(s) have run without an active ` +
          `plan. Call azoth_plan open with the problem, the criteria and at least ` +
          `${cfg.minimumCandidates} candidate paths (each with cost, assumptions, categories ` +
          'and falsifiers), then azoth_plan choose. A single lookup is free.',
        info: { name: 'AzothPlanRequired', code: 'AZOTH_PLAN_REQUIRED' },
      }
    }
    return undefined
  }
  const trips = holdingTrips(state)
  if (trips.length) {
    const lines = trips.slice(0, 6).map((trip) => {
      const subject = trip.assumption
        ? `assumption "${trip.assumption}"${trip.text ? ` (${trip.text})` : ''}`
        : `the chosen path in ${trip.decision}`
      return `- ${trip.id} ${trip.category}: ${subject} was falsified by ${trip.signal}${
        trip.calcId ? ` in ${trip.calcId}` : ''
      }.`
    })
    const decision = trips[0].decision
    const assumption = trips.find((trip) => trip.assumption)?.assumption
    const recovery = assumption
      ? `Call azoth_plan reopen {"decision":"${decision}","reason":...,"evidence":...} and choose ` +
        `again, or azoth_plan resolve {"assumption":"${assumption}","evidence":...} if it survived.`
      : `Call azoth_plan reopen {"decision":"${decision}","reason":...,"evidence":...} and choose again.`
    return {
      reason: [
        `azoth_calc is held: decision "${decision}" is tripped.`,
        ...lines,
        recovery,
        'Further calculations are refused until the decision is re-chosen or the trip is resolved.',
      ].join('\n'),
      info: { name: 'AzothPlanHeld', code: 'AZOTH_PLAN_HELD' },
    }
  }
  const overrun = stepOverrun(state)
  if (overrun) {
    return {
      reason:
        `azoth_calc is held: step ${overrun.step.id} "${overrun.step.description}" has run ` +
        `${overrun.elapsedMinutes.toFixed(0)} min against a ${overrun.step.time_box_minutes}-` +
        `minute box. Call azoth_plan review with extend_minutes to extend it, or ` +
        `fallback_started: true and the fallback${
          overrun.step.fallback ? ` (${overrun.step.fallback})` : ''
        }.`,
      info: { name: 'AzothPlanOverrun', code: 'AZOTH_STEP_OVERRUN' },
    }
  }
  const current = activeDecision(state)
  const due = reviewDue(state, cfg)
  if (current && current.choice && due) {
    const detail =
      due.kind === 'calculations'
        ? `${due.count} calculations since the last review`
        : `${due.minutes.toFixed(0)} minutes since the last review`
    return {
      reason:
        `azoth_calc is held for a checkpoint: ${detail} of decision "${current.id}". Call ` +
        `azoth_plan review {"decision":"${current.id}","verdict":"confirm",...} with at_risk, ` +
        'remaining_minutes and evidence, or verdict "reconsider" to reopen it.',
      info: { name: 'AzothPlanCheckpoint', code: 'AZOTH_PLAN_CHECKPOINT' },
    }
  }
  return undefined
}

function activeStep(state) {
  if (!state.activeStepId) return null
  const step = state.steps.find((item) => item.id === state.activeStepId)
  return step && step.status === 'active' ? step : null
}

function stepOverrun(state) {
  const step = activeStep(state)
  if (!step || !step.time_box_minutes || step.fallback_started) return null
  const elapsedMinutes = (Date.now() - (step.startedAt ?? Date.now())) / 60_000
  if (elapsedMinutes < step.time_box_minutes) return null
  return { step, elapsedMinutes }
}

function reviewDue(state, cfg) {
  const current = activeDecision(state)
  if (!current || !current.choice) return null
  if (cfg.reviewAfterCalcs > 0 && state.calcSinceReview >= cfg.reviewAfterCalcs) {
    return { kind: 'calculations', count: state.calcSinceReview }
  }
  if (cfg.reviewAfterMinutes > 0) {
    const elapsed = (Date.now() - (state.reviewWindowStartedAt ?? Date.now())) / 60_000
    if (elapsed >= cfg.reviewAfterMinutes) return { kind: 'minutes', minutes: elapsed }
  }
  return null
}

// ---------------------------------------------------------------------------------------
// external computation
//
// A bash command that runs an interpreter is a calculation outside azoth. It runs only
// under a permit whose command matches and whose purpose and verification the user has
// approved; the result is logged and bannered so the report can say where the number came
// from.

function activeChoice(state) {
  const decision = activeDecision(state)
  return decision?.choice ? decision : null
}

function numberOrNull(value) {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? number : null
}

function findPermit(state, id) {
  return state.permits.find((permit) => permit.id === id)
}

function permitMatches(permit, command, stepId) {
  if (!permit || (permit.status !== 'requested' && permit.status !== 'approved')) return false
  const target = String(command ?? '').trim()
  const wanted = String(permit.command ?? '').trim()
  if (!target || !wanted) return false
  if (permit.prefix ? !target.startsWith(wanted) : target !== wanted) return false
  if (permit.scope === 'step') return Boolean(permit.step) && permit.step === stepId
  return true
}

function matchingPermit(state, command, callId) {
  const stepId = state.activeStepId ?? null
  const found = state.permits.filter((permit) => {
    if (!permitMatches(permit, command, stepId)) return false
    if (
      permit.scope === 'once' &&
      permit.status === 'approved' &&
      permit.approvedCallId !== callId
    ) {
      return false
    }
    return true
  })
  return found.find((permit) => permit.status === 'approved') ?? found[0] ?? null
}

function markPermitApproved(state, permitId, callId, by) {
  const permit = findPermit(state, permitId)
  if (!permit || permit.status !== 'requested') return
  permit.status = 'approved'
  permit.approvedAt = now()
  permit.approvedBy = by
  permit.approvedCallId = callId ?? null
  state.updatedAt = now()
}

function consumePermit(state, permit, callId) {
  if (!permit) return
  permit.usedCallId = callId ?? null
  permit.uses = (permit.uses ?? 0) + 1
  if (permit.scope === 'once') permit.status = 'consumed'
}

function externalOutcome(result) {
  if (result?.isError) {
    return { exitCode: null, timedOut: false, kind: 'error', failed: true }
  }
  const value = result?.value ?? {}
  const kind = typeof value.kind === 'string' ? value.kind : 'foreground'
  const exitCode = Number.isInteger(value.exitCode) ? value.exitCode : null
  const timedOut = value.timedOut === true
  return {
    exitCode,
    timedOut,
    kind,
    failed: timedOut || (exitCode !== null && exitCode !== 0),
  }
}

function externalBanner(permit, command, outcome) {
  const who = permit?.approvedBy ? `approved by ${permit.approvedBy}` : 'not approved'
  const exit = outcome.exitCode === null ? '' : `; exit: ${outcome.exitCode}`
  const job = outcome.kind === 'promoted' ? '; promoted to a background job' : ''
  const failed = outcome.failed ? ' — FAILED; record the failure' : ''
  return (
    `EXTERNAL — not an azoth result; permit ${permit?.id ?? '?'} ` +
    `(${permit?.scope ?? '?'}, ${who}); verification: ${permit?.verification ?? 'not recorded'}` +
    `${exit}${job}${failed}\ncommand: ${String(command).slice(0, 200)}`
  )
}

function bashGate(exec, cfg) {
  const session = exec.agent?.session
  if (!session) return null
  const command = String(exec.arguments?.command ?? '')
  const kind = classifyCommand(command)
  if (kind === 'readonly' || kind === 'neutral' || kind === 'sanctioned-read') return null
  const state = planFor(session)
  const hold = holdReason(state, cfg)
  if (kind === 'sanctioned-compute') {
    if (cfg.gateSanctionedBridge && !state.decisions.length) {
      return {
        kind: 'deny',
        reason:
          'azoth is being called through bash. Open a plan with azoth_plan (or use azoth_calc) ' +
          'so the decision and its assumptions are recorded.',
        info: { name: 'AzothPlanRequired', code: 'AZOTH_PLAN_REQUIRED' },
      }
    }
    return hold ? { kind: 'deny', ...hold } : null
  }
  if (!state.decisions.length) {
    return {
      kind: 'deny',
      reason:
        'External computation is refused: no plan with a chosen path. Call azoth_plan open and ' +
        'choose first, then azoth_plan permit with the command, purpose and verification.',
      info: { name: 'AzothExternalPlanRequired', code: 'AZOTH_EXTERNAL_PLAN_REQUIRED' },
    }
  }
  if (hold) return { kind: 'deny', ...hold }
  if (!activeChoice(state)) {
    return {
      kind: 'deny',
      reason:
        'External computation is refused: the plan has no chosen path. Call azoth_plan choose ' +
        'first, then azoth_plan permit with the command, purpose and verification.',
      info: { name: 'AzothExternalPlanRequired', code: 'AZOTH_EXTERNAL_PLAN_REQUIRED' },
    }
  }
  const permit = matchingPermit(state, command, exec.callId)
  if (!permit) {
    return {
      kind: 'deny',
      reason:
        `External computation is refused: \`${command.slice(0, 200)}\` has no permit. Call ` +
        'azoth_plan permit {"command":...,"scope":"once|step|session","purpose":...,' +
        '"verification":...}; a matching command then asks the user to approve it. Use azoth_calc ' +
        'for any number azoth can calculate.',
      info: { name: 'AzothExternalPermitRequired', code: 'AZOTH_EXTERNAL_PERMIT_REQUIRED' },
    }
  }
  if (permit.status === 'approved') {
    externalRuns.set(exec, { permitId: permit.id, command, mode: 'approved' })
    return null
  }
  if (cfg.externalCompute === 'allow') {
    externalRuns.set(exec, { permitId: permit.id, command, mode: 'launcher' })
    return null
  }
  if (cfg.externalCompute !== 'ask') {
    return {
      kind: 'deny',
      reason:
        `External computation is refused by [plan].external_compute = "${cfg.externalCompute}". ` +
        `Permit ${permit.id} covers \`${command.slice(0, 120)}\`; relaunch with ` +
        '--allow-external-compute to approve it for this session.',
      info: {
        name: 'AzothExternalApprovalUnavailable',
        code: 'AZOTH_EXTERNAL_APPROVAL_UNAVAILABLE',
      },
    }
  }
  externalRuns.set(exec, { permitId: permit.id, command, mode: 'user' })
  return {
    kind: 'ask',
    reason: `azoth-plan: external computation ${permit.id}: ${command.slice(0, 300)}`,
    displayReason: {
      en:
        `Run an external calculation? ${permit.purpose} ` +
        `Verification: ${permit.verification}. Command: ${command.slice(0, 160)}`,
    },
  }
}

// ---------------------------------------------------------------------------------------
// actions

function reduce(state, args, context) {
  const cfg = context.cfg
  const action = args?.action
  switch (action) {
    case 'open':
      return openPlan(state, args, context, cfg)
    case 'add':
      return addDecision(state, args, context, cfg)
    case 'choose':
      return choose(state, args, context)
    case 'step':
      return recordStep(state, args, context)
    case 'basis':
      return recordBasis(state, args, context)
    case 'feasibility':
      return recordFeasibility(state, args)
    case 'fail':
      return recordFailure(state, args, context)
    case 'target':
      return recordTarget(state, args)
    case 'compare':
      return recordComparison(state, args)
    case 'job':
      return recordJob(state, args)
    case 'trip':
      return recordTrip(state, args, context)
    case 'reopen':
      return reopen(state, args, context)
    case 'resolve':
      return resolve(state, args, context, cfg)
    case 'review':
      return review(state, args, context, cfg)
    case 'permit':
      return recordPermit(state, args, context)
    case 'close':
      return closePlan(state, args, context)
    case 'status':
      return { message: renderState(state) }
    default:
      throw new Error(
        'azoth_plan: `action` must be open, add, choose, step, basis, feasibility, fail, ' +
          'target, compare, job, trip, reopen, resolve, review, permit, close or status',
      )
  }
}

function openPlan(state, args, context, cfg) {
  if (state.decisions.some((decision) => decision.status !== 'closed')) {
    throw new Error('azoth_plan: a plan is already active; close it before opening another')
  }
  const problem = text(args.problem, 'problem', 2000)
  const criteria = stringList(args.criteria, 'criteria', 1, 10, 500)
  state.questions = []
  const candidates = validateCandidates(state, args.candidates, cfg, new Set(), new Set())
  const questions = parseQuestions(state, args.questions, 'D1', candidates)
  attachQuestionAnswers(candidates, questions)
  requireStudyFields(state, candidates, questions)
  state.problem = problem
  state.criteria = criteria
  state.decisions = [createDecision('D1', null, 'concept selection', candidates)]
  state.steps = []
  state.trips = []
  state.reviews = []
  state.permits = []
  state.externalLog = []
  state.questions = questions
  state.basis = []
  state.failures = []
  state.targets = []
  state.comparisons = []
  state.jobs = []
  state.activeStepId = null
  state.ceremony = questions.length ? 'study' : 'simple'
  state.reviewWindowStartedAt = Date.now()
  state.calcSinceReview = 0
  state.closedAt = null
  state.closeReason = null
  state.updatedAt = now()
  return {
    message:
      `opened D1 (concept selection) with ${candidates.length} candidates; ` +
      'call azoth_plan choose with the best path and why',
  }
}

function addDecision(state, args, context, cfg) {
  if (!state.decisions.length || state.decisions.every((decision) => decision.status === 'closed')) {
    throw new Error('azoth_plan: open a plan before adding a decision')
  }
  if (state.decisions.length >= MAX_DECISIONS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_DECISIONS} decisions`)
  }
  const parentId = args.parent ? text(args.parent, 'parent', 40) : state.decisions[0].id
  const parent = findDecision(state, parentId)
  if (!parent) throw new Error(`azoth_plan: no decision "${parentId}"`)
  const parentState = decisionState(state, parent)
  if (parentState !== 'chosen') {
    throw new Error(`azoth_plan: choose the parent decision "${parentId}" first; it is ${parentState}`)
  }
  const question = text(args.question, 'question', 2000)
  const decisionId = nextId(state.decisions, 'D')
  const candidates = validateCandidates(state, args.candidates, cfg, assumptionIds(state))
  const questions = parseQuestions(state, args.questions, decisionId, candidates)
  attachQuestionAnswers(candidates, questions)
  requireStudyFields(state, candidates, questions)
  const decision = createDecision(decisionId, parentId, question, candidates)
  state.decisions.push(decision)
  state.questions.push(...questions)
  state.updatedAt = now()
  return {
    message: `added ${decision.id} under ${parentId} with ${candidates.length} candidates; call azoth_plan choose`,
  }
}

function choose(state, args, context) {
  const id = text(args.decision, 'decision', 40)
  const decision = findDecision(state, id)
  if (!decision) throw new Error(`azoth_plan: no decision "${id}"`)
  const stateName = decisionState(state, decision)
  if (stateName !== 'open' && stateName !== 'stale') {
    throw new Error(`azoth_plan: decision "${id}" is ${stateName}; only an open decision can be chosen`)
  }
  if (decision.parent) {
    const parent = findDecision(state, decision.parent)
    const parentState = decisionState(state, parent)
    if (parentState !== 'chosen') {
      throw new Error(`azoth_plan: choose the parent decision "${parent.id}" first; it is ${parentState}`)
    }
  }
  const candidateId = text(args.candidate, 'candidate', 80)
  const candidate = decision.candidates.find((item) => item.id === candidateId)
  if (!candidate) {
    throw new Error(
      `azoth_plan: decision "${id}" has no candidate "${candidateId}" (${decision.candidates
        .map((item) => item.id)
        .join(', ')})`,
    )
  }
  const bad = candidate.assumptions.find(
    (assumption) => assumptionStatus(state, assumption.id) === 'falsified',
  )
  if (bad) {
    throw new Error(
      `azoth_plan: candidate "${candidateId}" depends on falsified assumption "${bad.id}"; ` +
        `choose another path or resolve "${bad.id}" with evidence`,
    )
  }
  const questions = state.questions.filter(
    (item) => item.decision === id && item.priority === 'primary',
  )
  for (const item of questions) {
    if (item.answer_by !== candidateId) {
      throw new Error(
        `azoth_plan: candidate "${candidateId}" does not answer ${item.id} ("${item.text}"); ` +
          `choose "${item.answer_by}" or change the question's answer_by`,
      )
    }
    if (!candidate.fidelities.includes(item.fidelity)) {
      throw new Error(
        `azoth_plan: candidate "${candidateId}" does not provide ${item.fidelity} for ${item.id}`,
      )
    }
  }
  const rationale = text(args.rationale, 'rationale', 4000)
  const revision = decision.revisions.filter((item) => item.event === 'choose').length + 1
  decision.choice = {
    candidate: candidateId,
    rationale,
    revision,
    at: now(),
    callId: context.callId ?? null,
  }
  decision.revisions.push({ ...decision.choice, event: 'choose' })
  decision.status = 'chosen'
  state.calcSinceReview = 0
  state.reviewWindowStartedAt = Date.now()
  state.updatedAt = now()
  return { message: `chose ${candidateId} for ${id} (revision ${revision})` }
}

function latestBasis(state) {
  const latest = new Map()
  for (const item of state.basis) latest.set(item.check, item)
  return latest
}

function missingBasis(state, kind) {
  const required = REQUIRED_BASIS[kind] ?? []
  const latest = latestBasis(state)
  return required.filter((check) => {
    const item = latest.get(check)
    return !item || item.verdict === 'fail'
  })
}

const KIND_FIDELITY = {
  thermo: ['thermo'],
  steady: ['steady'],
  transport: ['steady'],
  transient: ['transient', 'field', 'olga'],
  field: ['field'],
  solver: ['steady', 'transient', 'field', 'olga'],
}

function parseFeasibility(value, field) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(
      `azoth_plan: \`${field}\` must be {dt_s, cost_per_step_ms, inversion_conditioning, ` +
        'singular_closures, estimated_minutes, fallback, success_test}',
    )
  }
  const dt = Number(value.dt_s)
  const cost = Number(value.cost_per_step_ms)
  const estimated = Number(value.estimated_minutes)
  if (!Number.isFinite(dt) || dt <= 0) {
    throw new Error(`azoth_plan: \`${field}.dt_s\` must be a positive number`)
  }
  if (!Number.isFinite(cost) || cost <= 0) {
    throw new Error(`azoth_plan: \`${field}.cost_per_step_ms\` must be a positive number`)
  }
  if (!Number.isFinite(estimated) || estimated <= 0) {
    throw new Error(`azoth_plan: \`${field}.estimated_minutes\` must be a positive number`)
  }
  return {
    dt_s: dt,
    cost_per_step_ms: cost,
    inversion_conditioning: text(
      value.inversion_conditioning,
      `${field}.inversion_conditioning`,
      500,
    ),
    singular_closures: text(value.singular_closures, `${field}.singular_closures`, 500),
    estimated_minutes: estimated,
    fallback: text(value.fallback, `${field}.fallback`, 500),
    success_test: text(value.success_test, `${field}.success_test`, 500),
  }
}

function updateStep(state, args) {
  const id = text(args.step, 'step', 40)
  const step = state.steps.find((item) => item.id === id)
  if (!step) throw new Error(`azoth_plan: no step "${id}"`)
  const status = args.status
  if (!STEP_STATUSES.includes(status)) {
    throw new Error(`azoth_plan: step status must be one of ${STEP_STATUSES.join(', ')}`)
  }
  step.status = status
  if (status !== 'active') step.finishedAt = Date.now()
  if (status === 'done' || status === 'failed') {
    for (const permit of state.permits) {
      if (permit.step === id && permit.status !== 'revoked') permit.status = 'revoked'
    }
    if (state.activeStepId === id) state.activeStepId = null
  }
  state.updatedAt = now()
  return { message: `step ${id} is ${status}` }
}

function recordStep(state, args, context) {
  if (args.step !== undefined && args.status !== undefined) return updateStep(state, args)
  const active = activeDecision(state)
  if (!active) throw new Error('azoth_plan: choose a decision before recording a step')
  if (state.steps.length >= MAX_STEPS) throw new Error(`azoth_plan: a plan is limited to ${MAX_STEPS} steps`)
  const description = text(args.description, 'description', 2000)
  const kind = args.kind === undefined ? 'check' : args.kind
  if (!STEP_KINDS.includes(kind)) {
    throw new Error(`azoth_plan: step kind must be one of ${STEP_KINDS.join(', ')}`)
  }
  const depends = args.depends_on === undefined
    ? []
    : stringList(args.depends_on, 'depends_on', 0, 20, 40)
  for (const id of depends) {
    if (!findDecision(state, id) && !findAssumption(state, id)) {
      throw new Error(`azoth_plan: step depends on unknown id "${id}"`)
    }
  }
  const timeBox = args.time_box_minutes === undefined
    ? kind === 'solver'
      ? 40
      : 0
    : Number(args.time_box_minutes)
  if (!Number.isFinite(timeBox) || timeBox < 0) {
    throw new Error('azoth_plan: `time_box_minutes` must be a non-negative number')
  }
  const estimated = args.estimated_minutes === undefined ? null : Number(args.estimated_minutes)
  if (estimated !== null && (!Number.isFinite(estimated) || estimated <= 0)) {
    throw new Error('azoth_plan: `estimated_minutes` must be a positive number')
  }
  if (estimated !== null && timeBox > 0 && estimated > timeBox && args.accept_overrun !== true) {
    throw new Error(
      `azoth_plan: the estimate (${estimated} min) exceeds the ${timeBox}-minute box; ` +
        'shorten it or pass accept_overrun: true to record the overrun',
    )
  }
  const overrunAccepted = estimated !== null && timeBox > 0 && estimated > timeBox
  if (state.ceremony === 'study' && REQUIRED_BASIS[kind]) {
    const missing = missingBasis(state, kind)
    if (missing.length) {
      throw new Error(
        `azoth_plan: a ${kind} step needs basis checks first (${missing.join(', ')}); ` +
          'call azoth_plan basis with the check, verdict and evidence',
      )
    }
    if (KIND_FIDELITY[kind]) {
      const questionId = text(args.question, 'question', 40)
      const question = state.questions.find(
        (item) => item.id === questionId && item.decision === active.id,
      )
      if (!question) {
        throw new Error(
          `azoth_plan: a ${kind} step needs a question of ${active.id} that it answers`,
        )
      }
      if (!KIND_FIDELITY[kind].includes(question.fidelity)) {
        throw new Error(
          `azoth_plan: ${question.id} is a ${question.fidelity} question; a ${kind} step ` +
            `answers ${KIND_FIDELITY[kind].join(', ')}`,
        )
      }
      if (question.answer_by !== active.choice.candidate) {
        throw new Error(
          `azoth_plan: the chosen candidate "${active.choice.candidate}" does not answer ` +
            `${question.id} (${question.answer_by} does)`,
        )
      }
    }
  }
  let feasibility = null
  if (kind === 'solver') {
    if (args.feasibility === undefined) {
      throw new Error(
        'azoth_plan: a solver step needs feasibility {dt_s, cost_per_step_ms, ' +
          'inversion_conditioning, singular_closures, estimated_minutes, fallback, success_test}',
      )
    }
    feasibility = parseFeasibility(args.feasibility, 'feasibility')
  } else if (args.feasibility !== undefined) {
    feasibility = parseFeasibility(args.feasibility, 'feasibility')
  }
  const step = {
    id: nextId(state.steps, 'S'),
    description,
    kind,
    depends_on: depends,
    time_box_minutes: timeBox,
    success_test: optionalText(args.success_test, 'success_test', 500),
    fallback: optionalText(args.fallback, 'fallback', 500),
    estimated_minutes: estimated,
    budget_note: overrunAccepted
      ? `accepted overrun: estimate ${estimated} min against a ${timeBox}-minute box`
      : null,
    feasibility,
    startedAt: Date.now(),
    finishedAt: null,
    status: 'active',
    fallback_started: false,
    at: now(),
    callId: context.callId ?? null,
    stale: false,
  }
  state.steps.push(step)
  state.activeStepId = step.id
  state.updatedAt = now()
  return {
    message:
      `recorded ${step.id} (${kind}${timeBox ? `, ${timeBox} min box` : ''}): ${description}`,
  }
}

function recordFeasibility(state, args) {
  const id = text(args.step, 'step', 40)
  const step = state.steps.find((item) => item.id === id)
  if (!step) throw new Error(`azoth_plan: no step "${id}"`)
  if (!['solver', 'transport', 'transient'].includes(step.kind)) {
    throw new Error(`azoth_plan: ${id} is a ${step.kind} step; feasibility is for a solver`)
  }
  step.feasibility = parseFeasibility(args, 'feasibility')
  state.updatedAt = now()
  return { message: `recorded feasibility for ${id}` }
}

function recordBasis(state, args, context) {
  const decision = activeDecision(state)
  if (!decision) throw new Error('azoth_plan: open a plan before a basis check')
  const check = args.check
  if (!BASIS_CHECKS.includes(check)) {
    throw new Error(`azoth_plan: basis check must be one of ${BASIS_CHECKS.join(', ')}`)
  }
  const verdict = args.verdict
  if (!BASIS_VERDICTS.includes(verdict)) {
    throw new Error(`azoth_plan: basis verdict must be one of ${BASIS_VERDICTS.join(', ')}`)
  }
  const evidence = text(args.evidence, 'evidence', 4000)
  if (state.basis.length >= MAX_BASIS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_BASIS} basis checks`)
  }
  const step = args.step !== undefined ? text(args.step, 'step', 40) : state.activeStepId
  const record = {
    id: nextId(state.basis, 'B'),
    check,
    verdict,
    evidence,
    decision: decision.id,
    step: step ?? null,
    at: now(),
  }
  state.basis.push(record)
  if (verdict === 'fail') {
    addTrip(state, {
      decision: decision.id,
      assumption: null,
      text: `basis check ${check} failed`,
      category: check === 'consistency' ? 'consistency' : 'validity',
      signal: 'basis',
      evidence,
      severity: 'trip',
      automatic: false,
      callId: context.callId ?? null,
    })
  }
  state.updatedAt = now()
  return { message: `recorded ${record.id}: ${check} ${verdict}` }
}

// ---------------------------------------------------------------------------------------
// failures, targets and jobs

function failureSignature(value) {
  return String(value ?? '')
    .toLowerCase()
    .replace(/\d+(?:\.\d+)?/g, '#')
    .replace(/[^a-z#_ ]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 120)
}

function externalFailure(result) {
  if (result?.isError) {
    const message = String(result.error?.message ?? 'external command failed')
    return { symptom: message.slice(0, 500), evidence: message.slice(0, 4000) }
  }
  const value = result?.value ?? {}
  const tail = `${value.stderr?.text ?? ''}\n${value.stdout?.text ?? ''}`.trim()
  const lines = tail.split('\n').map((line) => line.trim()).filter(Boolean)
  const symptom = (lines[lines.length - 1] ?? `exit ${value.exitCode ?? 'error'}`).slice(0, 500)
  return { symptom, evidence: (tail || symptom).slice(0, 4000) }
}

function failureStreak(state, failure, context) {
  if (!context.cfg.failureTripAfter) return null
  const matches = state.failures.filter(
    (item) =>
      item.signature === failure.signature &&
      (item.step === failure.step || item.candidate === failure.candidate),
  )
  if (matches.length < context.cfg.failureTripAfter) return null
  const evidence =
    `${matches.length} failures with signature "${failure.signature}": ` +
    matches
      .slice(-5)
      .map((item) => `${item.id} ${item.symptom}`)
      .join('; ')
  let trip = state.trips.find(
    (item) =>
      item.signal === 'failure_streak' &&
      item.decision === failure.decision &&
      item.detail?.signature === failure.signature,
  )
  if (trip) {
    trip.status = 'open'
    trip.evidence = evidence
    trip.detail = { ...trip.detail, failureIds: matches.map((item) => item.id) }
    trip.at = now()
    trip.calcSeq = state.calcCount
  } else {
    trip = addTrip(state, {
      decision: failure.decision,
      assumption: null,
      text: `repeated failure: ${failure.signature}`,
      category: 'feasibility',
      signal: 'failure_streak',
      evidence,
      severity: 'trip',
      automatic: true,
      callId: context.callId ?? null,
      detail: {
        signature: failure.signature,
        failureIds: matches.map((item) => item.id),
      },
    })
  }
  return { count: matches.length, trip }
}

function recordFailure(state, args, context, source = 'manual') {
  const decision = activeChoice(state) ?? activeDecision(state)
  if (!decision) throw new Error('azoth_plan: record a failure under an open plan')
  const symptom = text(args.symptom, 'symptom', 500)
  const evidence = text(args.evidence, 'evidence', 4000)
  const signature =
    args.signature === undefined
      ? failureSignature(symptom)
      : text(args.signature, 'signature', 120)
  if (state.failures.length >= MAX_FAILURES) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_FAILURES} failures`)
  }
  const failure = {
    id: nextId(state.failures, 'F'),
    decision: decision.id,
    candidate: decision.choice?.candidate ?? null,
    step:
      args.step !== undefined ? text(args.step, 'step', 40) : (state.activeStepId ?? null),
    symptom,
    signature,
    diagnosis: optionalText(args.diagnosis, 'diagnosis', 2000),
    evidence,
    artefact: optionalText(args.artefact, 'artefact', 500),
    category: args.category === undefined ? 'consistency' : category(args.category),
    source,
    at: now(),
  }
  state.failures.push(failure)
  const streak = failureStreak(state, failure, context)
  state.updatedAt = now()
  return {
    message: streak
      ? `recorded ${failure.id}; ${streak.count} failures with this signature — tripped ` +
        `${decision.id} (${streak.trip.id})`
      : `recorded ${failure.id} (${source}): ${signature}`,
  }
}

function parseBand(value, field) {
  if (typeof value === 'number') {
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`azoth_plan: \`${field}\` must be a non-negative number`)
    }
    return { plus_minus: value }
  }
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    if (value.plus_minus !== undefined) {
      const plusMinus = Number(value.plus_minus)
      if (!Number.isFinite(plusMinus) || plusMinus < 0) {
        throw new Error(`azoth_plan: \`${field}.plus_minus\` must be a non-negative number`)
      }
      return { plus_minus: plusMinus }
    }
    const low = Number(value.low)
    const high = Number(value.high)
    if (!Number.isFinite(low) || !Number.isFinite(high) || low > high) {
      throw new Error(`azoth_plan: \`${field}\` must be {low, high} with low <= high`)
    }
    return { low, high }
  }
  throw new Error(`azoth_plan: \`${field}\` must be a number or {low, high}`)
}

function bandContains(target, value) {
  if (target.band.plus_minus !== undefined) {
    return Math.abs(value - target.value) <= target.band.plus_minus + 1e-9
  }
  return value >= target.band.low - 1e-9 && value <= target.band.high + 1e-9
}

function recordTarget(state, args) {
  const decision = activeChoice(state) ?? activeDecision(state)
  if (!decision) throw new Error('azoth_plan: record a target under an open plan')
  const value = Number(args.value)
  if (!Number.isFinite(value)) throw new Error('azoth_plan: `value` must be a finite number')
  if (state.targets.length >= MAX_TARGETS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_TARGETS} targets`)
  }
  const target = {
    id: nextId(state.targets, 'T'),
    value,
    unit: text(args.unit, 'unit', 80),
    band: parseBand(args.band, 'band'),
    source: text(args.source, 'source', 500),
    decision: decision.id,
    at: now(),
  }
  state.targets.push(target)
  state.updatedAt = now()
  return {
    message:
      `recorded ${target.id}: ${value} ${target.unit} against ${target.source}; compare at ` +
      'least two models, one inside the band and one outside, before confirming',
  }
}

function recordComparison(state, args) {
  const targetId = text(args.target, 'target', 40)
  const target = state.targets.find((item) => item.id === targetId)
  if (!target) throw new Error(`azoth_plan: no target "${targetId}"`)
  const model = text(args.model, 'model', 200)
  const value = Number(args.value)
  if (!Number.isFinite(value)) throw new Error('azoth_plan: `value` must be a finite number')
  const verdict = args.verdict
  if (verdict !== 'agree' && verdict !== 'disagree') {
    throw new Error('azoth_plan: `verdict` must be "agree" or "disagree"')
  }
  const inside = bandContains(target, value)
  if (verdict === 'agree' && !inside) {
    throw new Error(`azoth_plan: ${value} is outside the band; record verdict "disagree"`)
  }
  if (verdict === 'disagree' && inside) {
    throw new Error(`azoth_plan: ${value} is inside the band; record verdict "agree"`)
  }
  if (state.comparisons.length >= MAX_COMPARISONS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_COMPARISONS} comparisons`)
  }
  const comparison = {
    id: nextId(state.comparisons, 'M'),
    target: targetId,
    model,
    value,
    unit: optionalText(args.unit, 'unit', 80),
    deviation: value - target.value,
    verdict,
    evidence: optionalText(args.evidence, 'evidence', 4000),
    at: now(),
  }
  state.comparisons.push(comparison)
  state.updatedAt = now()
  const models = new Set(
    state.comparisons.filter((item) => item.target === targetId).map((item) => item.model),
  )
  return { message: `recorded ${comparison.id}: ${model} ${value} (${verdict}); ${models.size} model(s)` }
}

function targetGate(state, decisionId) {
  const targets = state.targets.filter((target) => target.decision === decisionId)
  for (const target of targets) {
    const comparisons = state.comparisons.filter((item) => item.target === target.id)
    const models = new Set(comparisons.map((item) => item.model))
    const agree = comparisons.filter((item) => item.verdict === 'agree').length
    const disagree = comparisons.filter((item) => item.verdict === 'disagree').length
    if (models.size < 2 || agree === 0 || disagree === 0) {
      return (
        `target ${target.id} (${target.value} ${target.unit}) needs two models and both an ` +
        `agreeing and a disagreeing comparison; it has ${models.size} model(s), ${agree} ` +
        `agree and ${disagree} disagree`
      )
    }
  }
  return null
}

function targetSummary(state, decisionId) {
  return state.targets
    .filter((target) => target.decision === decisionId)
    .map((target) => {
      const comparisons = state.comparisons.filter((item) => item.target === target.id)
      const values = comparisons.map((item) => item.value)
      return {
        target: target.id,
        models: [...new Set(comparisons.map((item) => item.model))],
        min: values.length ? Math.min(...values) : null,
        max: values.length ? Math.max(...values) : null,
        spread: values.length ? Math.max(...values) - Math.min(...values) : null,
        agree: comparisons.filter((item) => item.verdict === 'agree').length,
        disagree: comparisons.filter((item) => item.verdict === 'disagree').length,
      }
    })
}

function recordJob(state, args) {
  if (!state.decisions.length) throw new Error('azoth_plan: open a plan before a job')
  const jobId = text(args.job_id, 'job_id', 120)
  const purpose = text(args.purpose, 'purpose', 500)
  if (state.jobs.some((job) => job.job_id === jobId)) {
    return { message: `job ${jobId} is already recorded` }
  }
  if (state.jobs.length >= MAX_JOBS) {
    throw new Error(`azoth_plan: a plan is limited to ${MAX_JOBS} jobs`)
  }
  const job = {
    id: nextId(state.jobs, 'J'),
    job_id: jobId,
    purpose,
    step: args.step === undefined ? (state.activeStepId ?? null) : text(args.step, 'step', 40),
    label: optionalText(args.label, 'label', 300),
    status: 'running',
    startedAt: Date.now(),
    finishedAt: null,
    at: now(),
  }
  state.jobs.push(job)
  state.updatedAt = now()
  return { message: `recorded ${job.id} for job ${jobId}` }
}

function recordPermit(state, args, context) {
  if (!state.decisions.length) throw new Error('azoth_plan: open a plan before a permit')
  if (!activeChoice(state)) throw new Error('azoth_plan: choose a decision before a permit')
  const command = text(args.command, 'command', 500)
  const purpose = text(args.purpose, 'purpose', 500)
  const verification = text(args.verification, 'verification', 500)
  const scope = PERMIT_SCOPES.includes(args.scope) ? args.scope : 'step'
  const step =
    args.step !== undefined ? text(args.step, 'step', 40) : (state.activeStepId ?? null)
  if (scope === 'step' && !step) {
    throw new Error('azoth_plan: scope "step" needs an active step; record one with step first')
  }
  if (state.permits.length >= MAX_PERMITS) {
    throw new Error(`azoth_plan: at most ${MAX_PERMITS} permits per plan`)
  }
  const existing = state.permits.find(
    (permit) => permit.command === command && permit.scope === scope && permit.status !== 'revoked',
  )
  if (existing) {
    return { message: `permit ${existing.id} already covers \`${command}\` (${existing.status})` }
  }
  const permit = {
    id: nextId(state.permits, 'P'),
    command,
    prefix: args.prefix === true,
    scope,
    step,
    purpose,
    verification,
    expected_minutes: numberOrNull(args.expected_minutes),
    status: 'requested',
    requestedAt: now(),
    approvedAt: null,
    approvedBy: null,
    approvedCallId: null,
    usedCallId: null,
    uses: 0,
  }
  state.permits.push(permit)
  state.updatedAt = now()
  return {
    message:
      `requested ${permit.id} for \`${command}\` (${scope}); purpose: ${purpose}; ` +
      `verification: ${verification}. The next matching command asks the user to approve it.`,
  }
}

function recordTrip(state, args, context) {
  const categoryName = category(args.category)
  const evidence = text(args.evidence, 'evidence', 4000)
  const severity = args.severity === 'note' ? 'note' : 'trip'
  let decision
  let assumption = null
  let body = ''
  if (args.assumption !== undefined) {
    const found = findAssumption(state, text(args.assumption, 'assumption', 80))
    if (!found) throw new Error(`azoth_plan: no assumption "${args.assumption}"`)
    decision = found.decision
    assumption = found.assumption.id
    body = found.assumption.text
  } else {
    const id = text(args.decision, 'decision', 40)
    decision = findDecision(state, id)
    if (!decision) throw new Error(`azoth_plan: trip needs an assumption or a decision`)
  }
  if (decision.status === 'closed') {
    throw new Error(`azoth_plan: decision "${decision.id}" is closed`)
  }
  if (severity === 'trip' && !decision.choice && !assumption) {
    throw new Error('azoth_plan: a decision-level trip needs a chosen decision')
  }
  const trip = addTrip(state, {
    decision: decision.id,
    assumption,
    text: body,
    category: categoryName,
    signal: 'manual',
    evidence,
    severity,
    automatic: false,
    callId: context.callId ?? null,
  })
  state.updatedAt = now()
  return {
    message: `${severity === 'trip' ? 'tripped' : 'noted'} ${trip.id} (${categoryName}) on ${decision.id}${
      assumption ? `/${assumption}` : ''
    }`,
  }
}

function reopen(state, args, context) {
  const id = text(args.decision, 'decision', 40)
  const decision = findDecision(state, id)
  if (!decision) throw new Error(`azoth_plan: no decision "${id}"`)
  const stateName = decisionState(state, decision)
  if (stateName !== 'tripped' && stateName !== 'chosen') {
    throw new Error(`azoth_plan: reopen is for a chosen or tripped decision; "${id}" is ${stateName}`)
  }
  const reason = text(args.reason, 'reason', 4000)
  const evidence = text(args.evidence, 'evidence', 4000)
  for (const trip of state.trips) {
    if (
      trip.decision === id &&
      (trip.status === 'open' || trip.status === 'reopened') &&
      trip.severity === 'trip'
    ) {
      trip.status = 'reopened'
    }
  }
  decision.revisions.push({
    candidate: null,
    rationale: `reopened: ${reason}`,
    revision: decision.revisions.length + 1,
    at: now(),
    callId: context.callId ?? null,
    event: 'reopen',
    evidence,
  })
  decision.choice = null
  decision.status = 'open'
  const staleIds = new Set([id, ...descendants(state, id).map((child) => child.id)])
  for (const child of descendants(state, id)) child.status = 'stale'
  for (const step of state.steps) {
    const touches = (step.depends_on ?? []).some((item) => {
      if (staleIds.has(item)) return true
      const found = findAssumption(state, item)
      return found ? staleIds.has(found.decision.id) : false
    })
    if (touches) step.stale = true
  }
  state.calcSinceReview = 0
  state.reviewWindowStartedAt = Date.now()
  state.updatedAt = now()
  return { message: `reopened ${id}; call azoth_plan choose with a new path` }
}

function resolve(state, args, context, cfg) {
  const id = text(args.assumption, 'assumption', 80)
  const found = findAssumption(state, id)
  if (!found) throw new Error(`azoth_plan: no assumption "${id}"`)
  const open = state.trips.filter(
    (trip) => trip.assumption === id && (trip.status === 'open' || trip.status === 'reopened'),
  )
  if (!open.length) throw new Error(`azoth_plan: assumption "${id}" has no open trip`)
  const evidence = text(args.evidence, 'evidence', 4000)
  if (cfg.resolveRequiresCleanCalc) {
    const callId = text(args.calc_call, 'calc_call', 80)
    const latest = Math.max(...open.map((trip) => trip.calcSeq))
    const entry = state.calcLog.find((item) => item.callId === callId)
    if (
      !entry ||
      entry.isError ||
      entry.calcSeq <= latest ||
      entry.falsified.includes(id)
    ) {
      throw new Error(
        'azoth_plan: resolve needs a later clean calculation that did not falsify the assumption',
      )
    }
  }
  const at = now()
  for (const trip of open) {
    trip.status = 'resolved'
    trip.resolvedBy = {
      evidence,
      calcCall: args.calc_call ?? null,
      at,
      manualOverride: args.calc_call === undefined,
    }
  }
  state.calcSinceReview = 0
  state.reviewWindowStartedAt = Date.now()
  state.updatedAt = at
  return { message: `resolved ${id}; ${open.length} trip(s) closed with evidence` }
}

function review(state, args, context, cfg) {
  const id = text(args.decision, 'decision', 40)
  const decision = findDecision(state, id)
  if (!decision) throw new Error(`azoth_plan: no decision "${id}"`)
  const stateName = decisionState(state, decision)
  if (stateName !== 'chosen') {
    throw new Error(`azoth_plan: review is for a chosen decision; "${id}" is ${stateName}`)
  }
  if (args.verdict !== 'confirm' && args.verdict !== 'reconsider') {
    throw new Error('azoth_plan: review needs verdict "confirm" or "reconsider"')
  }
  const evidence = text(args.evidence, 'evidence', 4000)
  const atRisk =
    args.at_risk === undefined ? [] : stringList(args.at_risk, 'at_risk', 0, 10, 300)
  const remaining =
    args.remaining_minutes === undefined ? null : Number(args.remaining_minutes)
  if (remaining !== null && (!Number.isFinite(remaining) || remaining < 0)) {
    throw new Error('azoth_plan: `remaining_minutes` must be a non-negative number')
  }
  const fallbackStarted = args.fallback_started === true
  const extend = args.extend_minutes === undefined ? 0 : Number(args.extend_minutes)
  if (!Number.isFinite(extend) || extend < 0) {
    throw new Error('azoth_plan: `extend_minutes` must be a non-negative number')
  }
  let ranking = null
  if (args.ranking !== undefined) {
    ranking = stringList(args.ranking, 'ranking', 1, MAX_CANDIDATES, 80)
    for (const candidateId of ranking) {
      if (!decision.candidates.some((candidate) => candidate.id === candidateId)) {
        throw new Error(`azoth_plan: ranking names unknown candidate "${candidateId}"`)
      }
    }
    if (args.verdict === 'confirm' && ranking[0] !== decision.choice.candidate) {
      throw new Error(
        `azoth_plan: the ranking puts "${ranking[0]}" first, not the chosen ` +
          `"${decision.choice.candidate}"; record verdict "reconsider" and reopen`,
      )
    }
  }
  const step = activeStep(state)
  const overrun = stepOverrun(state)
  if (overrun && args.verdict === 'confirm' && extend <= 0 && !fallbackStarted) {
    throw new Error(
      `azoth_plan: step ${overrun.step.id} is over its ${overrun.step.time_box_minutes}-minute ` +
        'box; pass extend_minutes to extend it or fallback_started: true',
    )
  }
  if (extend > 0) {
    if (!step) throw new Error('azoth_plan: extend_minutes needs an active step')
    step.time_box_minutes += extend
    step.extended_by = (step.extended_by ?? 0) + extend
  }
  if (fallbackStarted) {
    if (!step) throw new Error('azoth_plan: fallback_started needs an active step')
    step.fallback_started = true
  }
  if (args.verdict === 'confirm') {
    const gate = targetGate(state, id)
    if (gate) throw new Error(`azoth_plan: ${gate}`)
  }
  state.reviews.push({
    decision: id,
    verdict: args.verdict,
    evidence,
    ranking,
    at_risk: atRisk,
    remaining_minutes: remaining,
    fallback_started: fallbackStarted,
    extended_minutes: extend || null,
    targets: targetSummary(state, id),
    at: now(),
    callId: context.callId ?? null,
  })
  state.calcSinceReview = 0
  state.reviewWindowStartedAt = Date.now()
  if (args.verdict === 'reconsider') {
    const categoryName = args.category === undefined ? 'judgment' : category(args.category)
    addTrip(state, {
      decision: id,
      assumption: null,
      text: `review reconsidered ${id}`,
      category: categoryName,
      signal: 'review',
      evidence,
      severity: 'trip',
      automatic: false,
      callId: context.callId ?? null,
    })
    state.updatedAt = now()
    return {
      message: `review reconsidered ${id} (${categoryName}); call azoth_plan reopen`,
    }
  }
  state.updatedAt = now()
  return { message: `review confirmed ${id}; checkpoint reset` }
}

function closePlan(state, args) {
  const reason = text(args.reason, 'reason', 4000)
  const open = holdingTrips(state)
  if (open.length && args.force !== true) {
    throw new Error(
      `azoth_plan: ${open.length} trip(s) hold the plan; resolve or reopen first, or pass ` +
        'force: true with a reason to abandon it',
    )
  }
  if (args.force !== true) {
    for (const decision of state.decisions) {
      const gate = targetGate(state, decision.id)
      if (gate) {
        throw new Error(`azoth_plan: ${gate}; pass force: true with a reason to abandon`)
      }
    }
  }
  for (const trip of open) trip.status = 'superseded'
  for (const decision of state.decisions) decision.status = 'closed'
  state.closedAt = now()
  state.closeReason = reason
  state.updatedAt = now()
  return { message: 'plan closed; a later azoth_plan open starts a new record' }
}

// ---------------------------------------------------------------------------------------
// rendering

function serialize(state) {
  return {
    v: state.v,
    problem: state.problem,
    criteria: state.criteria,
    decisions: state.decisions.map((decision) => ({
      id: decision.id,
      parent: decision.parent,
      question: decision.question,
      status: decisionState(state, decision),
      choice: decision.choice
        ? {
            candidate: decision.choice.candidate,
            rationale: decision.choice.rationale,
            revision: decision.choice.revision,
          }
        : null,
      revisions: decision.revisions.slice(-20),
      candidates: decision.candidates.map((candidate) => ({
        id: candidate.id,
        path: candidate.path,
        cost: candidate.cost,
        assumptions: candidate.assumptions.map((assumption) => ({
          id: assumption.id,
          text: assumption.text,
          category: assumption.category,
          falsifier: assumption.falsifier,
          ...(assumption.calc ? { calc: assumption.calc } : {}),
          status: assumptionStatus(state, assumption.id),
        })),
      })),
    })),
    steps: state.steps.slice(-MAX_STEPS).map((step) => ({
      id: step.id,
      description: step.description,
      depends_on: step.depends_on,
      status: stepStale(state, step) ? 'stale' : 'open',
    })),
    trips: state.trips.slice(-MAX_TRIPS).map((trip) => ({
      id: trip.id,
      decision: trip.decision,
      assumption: trip.assumption,
      category: trip.category,
      signal: trip.signal,
      evidence: trip.evidence,
      severity: trip.severity,
      status: trip.status,
      calcId: trip.calcId,
      at: trip.at,
    })),
    reviews: state.reviews.slice(-20),
    permits: state.permits.slice(-MAX_PERMITS),
    externalLog: state.externalLog.slice(-20),
    questions: state.questions,
    basis: state.basis.slice(-MAX_BASIS),
    failures: state.failures.slice(-MAX_FAILURES),
    targets: state.targets,
    comparisons: state.comparisons.slice(-MAX_COMPARISONS),
    jobs: state.jobs,
    ceremony: state.ceremony,
    activeStepId: state.activeStepId,
    calcCount: state.calcCount,
    calcSinceReview: state.calcSinceReview,
  }
}

function compactState(state) {
  return {
    ...state,
    calcLog: state.calcLog.slice(-10),
    reviews: state.reviews.slice(-5),
    steps: state.steps.slice(-50).map((step) => ({
      ...step,
      description: step.description.slice(0, 300),
      success_test: step.success_test ? step.success_test.slice(0, 200) : null,
      fallback: step.fallback ? step.fallback.slice(0, 200) : null,
    })),
    trips: state.trips.slice(-100),
    permits: state.permits.slice(-8).map((permit) => ({
      ...permit,
      command: permit.command.slice(0, 200),
      purpose: permit.purpose.slice(0, 200),
      verification: permit.verification.slice(0, 300),
    })),
    externalLog: state.externalLog.slice(-10).map((entry) => ({
      ...entry,
      command: entry.command.slice(0, 200),
    })),
    basis: state.basis.slice(-10),
    failures: state.failures.slice(-15).map((failure) => ({
      ...failure,
      symptom: failure.symptom.slice(0, 200),
      evidence: failure.evidence.slice(0, 600),
      diagnosis: failure.diagnosis ? failure.diagnosis.slice(0, 300) : null,
    })),
    comparisons: state.comparisons.slice(-10),
    jobs: state.jobs.slice(-10),
    decisions: state.decisions.map((decision) => ({
      ...decision,
      revisions: decision.revisions.slice(-5),
      candidates: decision.candidates.map((candidate) => ({
        ...candidate,
        path: candidate.path.slice(0, 300),
        cost: candidate.cost.slice(0, 100),
        assumptions: candidate.assumptions.map((assumption) => ({
          ...assumption,
          text: assumption.text.slice(0, 300),
        })),
      })),
    })),
  }
}

function snapshotLine(state) {
  let body = ''
  try {
    body = JSON.stringify(state)
  } catch {
    body = JSON.stringify({ v: 1, decisions: [], error: 'state not serialisable' })
  }
  if (body.length > 40_000) {
    try {
      body = JSON.stringify(compactState(state))
    } catch {
      body = JSON.stringify({ v: 1, decisions: [], error: 'state not serialisable' })
    }
  }
  return `${SNAPSHOT_MARKER} ${body}`
}

function renderState(state) {
  if (!state.decisions.length) return 'azoth plan: no plan is open'
  const lines = [`azoth plan: ${state.problem || '(no problem stated)'}`]
  if (state.criteria.length) lines.push(`criteria: ${state.criteria.join(', ')}`)
  for (const decision of state.decisions) {
    const depth = ancestorIds(state, decision).length
    const indent = '  '.repeat(depth)
    const status = decisionState(state, decision)
    const choice = decision.choice
      ? ` — chosen: ${decision.choice.candidate} (revision ${decision.choice.revision})`
      : ''
    lines.push(`${indent}${decision.id} ${decision.question} [${status}]${choice}`)
    for (const candidate of decision.candidates) {
      lines.push(`${indent}  ${candidate.id} ${candidate.path} (${candidate.cost})`)
      for (const assumption of candidate.assumptions) {
        const binding = assumption.calc ? ` on ${assumption.calc}` : ''
        lines.push(
          `${indent}    ${assumption.id} ${assumption.text} ` +
            `(${assumption.category}/${assumption.falsifier}${binding}: ${assumptionStatus(
              state,
              assumption.id,
            )})`,
        )
      }
    }
  }
  if (state.steps.length) {
    lines.push('steps:')
    for (const step of state.steps) {
      const status = stepStale(state, step) ? 'stale' : (step.status ?? 'open')
      const depends = step.depends_on.length ? ` — depends on ${step.depends_on.join(', ')}` : ''
      const kind = step.kind ? ` ${step.kind}` : ''
      const box = step.time_box_minutes ? `, ${step.time_box_minutes}m box` : ''
      lines.push(`  ${step.id}${kind} ${step.description}${depends} [${status}${box}]`)
    }
  }
  const trips = state.trips.filter((trip) => trip.status !== 'resolved').slice(-20)
  if (trips.length) {
    lines.push('trips:')
    for (const trip of trips) {
      const subject = trip.assumption ?? trip.decision
      lines.push(
        `  ${trip.id} ${trip.decision}/${subject} ${trip.category} ${trip.signal} ` +
          `[${trip.status}/${trip.severity}] ${trip.evidence}`,
      )
    }
  }
  if (state.questions.length) {
    lines.push('questions:')
    for (const question of state.questions) {
      lines.push(
        `  ${question.id} ${question.text} ` +
          `(${question.fidelity}/${question.priority} -> ${question.answer_by})`,
      )
    }
  }
  if (state.basis.length) {
    const latest = [...latestBasis(state).values()]
    lines.push(`basis: ${latest.map((item) => `${item.check} ${item.verdict}`).join(', ')}`)
  }
  if (state.permits.length) {
    lines.push('external permits:')
    for (const permit of state.permits.slice(-10)) {
      lines.push(
        `  ${permit.id} ${permit.status} ${permit.scope} \`${permit.command}\` — ` +
          `${permit.purpose}; verification: ${permit.verification}`,
      )
    }
  }
  if (state.externalLog.length) {
    const recent = state.externalLog.slice(-5)
    lines.push(
      `external runs: ${state.externalLog.length} (` +
        recent.map((entry) => `${entry.permit}:${entry.exitCode ?? entry.kind}`).join(', ') +
        ')',
    )
  }
  if (state.failures.length) {
    const groups = new Map()
    for (const failure of state.failures) {
      const key = `${failure.candidate ?? '-'}/${failure.step ?? '-'}/${failure.signature}`
      const group = groups.get(key) ?? { count: 0, last: failure }
      group.count += 1
      group.last = failure
      groups.set(key, group)
    }
    lines.push('failures:')
    for (const [key, group] of [...groups.entries()].slice(-10)) {
      lines.push(`  ${group.count}x ${key} latest ${group.last.id}: ${group.last.symptom}`)
    }
  }
  if (state.targets.length) {
    lines.push('targets:')
    for (const target of state.targets) {
      const count = state.comparisons.filter((item) => item.target === target.id).length
      lines.push(
        `  ${target.id} ${target.value} ${target.unit} from ${target.source}; ` +
          `${count} comparison(s)`,
      )
    }
  }
  if (state.jobs.length) {
    const running = state.jobs.filter((job) => job.status === 'running')
    const names = running.slice(-3).map((job) => job.job_id)
    lines.push(`jobs: ${running.length} running${names.length ? ` (${names.join(', ')})` : ''}`)
  }
  if (state.calcSinceReview) {
    lines.push(`checkpoint: ${state.calcSinceReview} calculations since the last review`)
  }
  return lines.join('\n')
}

function statusFor(session) {
  const state = session ? planFor(session) : null
  if (!state || !state.decisions.length) return 'No plan is open.'
  const trips = holdingTrips(state)
  if (trips.length) {
    const trip = trips[0]
    return `PLAN HELD: ${trip.decision} ${trip.category} — ${trip.assumption ?? 'the chosen path'} (${trip.signal}). Call azoth_plan status, then reopen or resolve.`
  }
  const active = activeDecision(state)
  const current = state.decisions.find((decision) => decisionState(state, decision) === 'open')
  if (current) return `Plan: ${current.id} ${current.question} is open; call azoth_plan choose.`
  if (active) {
    const choice = active.choice?.candidate ?? '?'
    return `Plan: ${active.id} chose ${choice}; ${state.calcSinceReview} calculations since the last review.`
  }
  return 'Plan: no decision is chosen; call azoth_plan status.'
}

function renderPlan(_args, value) {
  const text = [value?.message ?? '', value?.record ?? ''].filter(Boolean).join('\n\n')
  return content(text)
}

// ---------------------------------------------------------------------------------------
// per-session state

function stateKey(session) {
  return session?.id ?? session?.header?.id ?? null
}

function normalizeState(value) {
  const state = createState()
  if (!value || typeof value !== 'object') return state
  Object.assign(state, value)
  state.v = 1
  for (const key of [
    'decisions',
    'steps',
    'trips',
    'reviews',
    'calcLog',
    'permits',
    'externalLog',
    'questions',
    'basis',
    'failures',
    'targets',
    'comparisons',
    'jobs',
  ]) {
    if (!Array.isArray(state[key])) state[key] = []
  }
  state.criteria = Array.isArray(state.criteria) ? state.criteria : []
  state.activeStepId = typeof state.activeStepId === 'string' ? state.activeStepId : null
  state.ceremony = state.ceremony === 'study' ? 'study' : 'simple'
  state.reviewWindowStartedAt = Number.isFinite(state.reviewWindowStartedAt)
    ? state.reviewWindowStartedAt
    : Date.now()
  state.calcCount = Number.isInteger(state.calcCount) ? state.calcCount : 0
  state.calcSinceReview = Number.isInteger(state.calcSinceReview) ? state.calcSinceReview : 0
  return state
}

function hydrate(session) {
  try {
    if (typeof session?.snapshotEvents !== 'function') return null
    const events = session.snapshotEvents()
    if (!Array.isArray(events)) return null
    for (let index = events.length - 1; index >= 0; index -= 1) {
      const line = findRecord(events[index], { depth: 0, nodes: 5000 })
      if (!line) continue
      const parsed = JSON.parse(line)
      if (parsed && parsed.v === 1 && Array.isArray(parsed.decisions)) {
        return normalizeState(parsed)
      }
    }
  } catch {
    return null
  }
  return null
}

function findRecord(value, state) {
  if (state.nodes <= 0 || state.depth > 12) return null
  state.nodes -= 1
  if (typeof value === 'string') {
    const at = value.indexOf(SNAPSHOT_MARKER)
    if (at < 0) return null
    const rest = value.slice(at + SNAPSHOT_MARKER.length)
    const newline = rest.indexOf('\n')
    return (newline < 0 ? rest : rest.slice(0, newline)).trim()
  }
  if (value === null || typeof value !== 'object') return null
  for (const child of Object.values(value)) {
    state.depth += 1
    const found = findRecord(child, state)
    state.depth -= 1
    if (found) return found
  }
  return null
}

function planFor(session) {
  const key = stateKey(session)
  if (key === null) return null
  let state = plans.get(key)
  if (state) return state
  state = hydrate(session) ?? createState()
  plans.set(key, state)
  return state
}

// ---------------------------------------------------------------------------------------
// plugin

function noticeMessage(text) {
  return {
    role: 'user',
    id: uuid(),
    content: [{ type: 'text', text }],
    source: {
      kind: 'azoth-plan',
      form: 'notice',
      summary: 'azoth plan tripped',
    },
  }
}

function prepend(decision, result, text) {
  if (decision.kind === 'block') {
    return { ...decision, feedback: [{ type: 'text', text }, ...(decision.feedback ?? [])] }
  }
  if (has(decision, 'value')) {
    return {
      ...decision,
      additionalContexts: [noticeMessage(text), ...(decision.additionalContexts ?? [])],
    }
  }
  return {
    kind: 'accept',
    content: [{ type: 'text', text }, ...(decision.content ?? result.content ?? [])],
    ...(decision.additionalContexts ? { additionalContexts: decision.additionalContexts } : {}),
  }
}

function tripBanner(trips) {
  const lines = ['AZOTH PLAN TRIPPED']
  for (const trip of trips.slice(0, 6)) {
    const subject = trip.assumption
      ? `assumption ${trip.assumption}${trip.text ? ` (${trip.text})` : ''}`
      : `the chosen path in ${trip.decision}`
    lines.push(
      `- ${trip.category}: ${subject} was falsified by ${trip.signal}${
        trip.calcId ? ` in ${trip.calcId}` : ''
      }`,
    )
    if (trip.evidence) lines.push(`  ${trip.evidence}`)
  }
  const assumption = trips.find((trip) => trip.assumption)?.assumption
  lines.push(
    assumption
      ? `Call azoth_plan status, then azoth_plan reopen with the decision, or azoth_plan resolve ` +
          `{"assumption":"${assumption}","evidence":...} if it survived.`
      : 'Call azoth_plan status, then azoth_plan reopen with the decision.',
  )
  lines.push('Further azoth_calc calls are held until the decision is re-chosen.')
  return lines.join('\n')
}

function checkpointBanner(state, cfg) {
  if (cfg.reviewAfterCalcs <= 0 || state.calcSinceReview < cfg.reviewAfterCalcs) return ''
  const active = activeDecision(state)
  if (!active) return ''
  return [
    'AZOTH PLAN CHECKPOINT',
    `${state.calcSinceReview} calculations since the last review of ${active.id}.`,
    `Call azoth_plan review {"decision":"${active.id}","verdict":"confirm","evidence":...} to ` +
      'reaffirm the choice, or verdict "reconsider" with evidence to reopen it.',
    'The next azoth_calc is held until the review is recorded.',
  ].join('\n')
}

export function apply(ctx, config = {}) {
  const cfg = normalizeConfig(config)
  const bodyRan = new WeakSet()

  const watched = (toolName) => toolName === 'azoth_calc' || toolName === 'azoth_intake'

  ctx.tools.register({
    name: 'azoth_plan',
    description:
      'Record a multi-step calculation plan as a decision tree. open the concept decision with ' +
      'at least two candidate paths and the assumptions each rests on; choose one; add a ' +
      'subordinate decision when a later choice matters; step records ordered work. Automatic ' +
      'checks watch azoth_calc and azoth_intake for numerical, validity and evidence failures. ' +
      'For any other reason — a model outside its physical basis, a balance that does not close, ' +
      'a missing capability, a changed requirement, a budget overrun, judgement — call trip with ' +
      'the category and evidence. A trip holds azoth_calc and reopens its decision. permit records ' +
      'an external bash calculation (command, scope, purpose, verification); an interpreter run ' +
      'outside azoth is refused until its permit is approved. review is the periodic checkpoint; ' +
      'status returns the record.',
    parameters: {
      type: 'object',
      additionalProperties: false,
      properties: {
        action: {
          type: 'string',
          enum: [
            'open',
            'add',
            'choose',
            'step',
            'basis',
            'feasibility',
            'fail',
            'target',
            'compare',
            'job',
            'trip',
            'reopen',
            'resolve',
            'review',
            'permit',
            'close',
            'status',
          ],
        },
        problem: { type: 'string' },
        criteria: { type: 'array', items: { type: 'string' } },
        candidates: {
          type: 'array',
          items: {
            type: 'object',
            additionalProperties: false,
            properties: {
              id: { type: 'string' },
              path: { type: 'string' },
              cost: { type: 'string', description: 'relative effort, risk and accuracy estimate' },
              estimate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  minutes: { type: 'number' },
                  compute: { type: 'string' },
                  risk: { type: 'string' },
                },
                required: ['minutes', 'compute', 'risk'],
              },
              fallback: { type: 'string' },
              answers: { type: 'array', items: { type: 'string' } },
              fidelities: { type: 'array', items: { type: 'string', enum: FIDELITIES } },
              assumptions: {
                type: 'array',
                items: {
                  type: 'object',
                  additionalProperties: false,
                  properties: {
                    id: { type: 'string' },
                    text: { type: 'string' },
                    category: { type: 'string', enum: CATEGORIES },
                    falsifier: { type: 'string', enum: FALSIFIERS },
                    calc: {
                      type: 'string',
                      description: 'an azoth_calc id, "*" for any calculation, or "intake"',
                    },
                  },
                  required: ['id', 'text', 'category', 'falsifier'],
                },
              },
            },
            required: ['id', 'path', 'cost', 'assumptions'],
          },
        },
        parent: { type: 'string' },
        question: { type: 'string' },
        decision: { type: 'string' },
        candidate: { type: 'string' },
        rationale: { type: 'string' },
        description: { type: 'string' },
        depends_on: { type: 'array', items: { type: 'string' } },
        assumption: { type: 'string' },
        category: { type: 'string', enum: CATEGORIES },
        evidence: { type: 'string' },
        severity: { type: 'string', enum: ['trip', 'note'] },
        verdict: {
          type: 'string',
          enum: ['confirm', 'reconsider', 'agree', 'disagree', 'pass', 'warn', 'fail'],
        },
        ranking: { type: 'array', items: { type: 'string' } },
        calc_call: { type: 'string' },
        reason: { type: 'string' },
        force: { type: 'boolean', description: 'close a held plan with a reason' },
        command: { type: 'string', description: 'the external bash command a permit covers' },
        prefix: { type: 'boolean', description: 'match command prefixes rather than exact text' },
        scope: { type: 'string', enum: PERMIT_SCOPES },
        purpose: { type: 'string', description: 'why the calculation is not an azoth call' },
        verification: { type: 'string', description: 'how the result is checked' },
        expected_minutes: { type: 'number' },
        step: { type: 'string' },
        questions: {
          type: 'array',
          items: {
            type: 'object',
            additionalProperties: false,
            properties: {
              text: { type: 'string' },
              fidelity: { type: 'string', enum: FIDELITIES },
              priority: { type: 'string', enum: PRIORITIES },
              answer_by: { type: 'string' },
            },
            required: ['text', 'fidelity', 'answer_by'],
          },
        },
        kind: { type: 'string', enum: STEP_KINDS },
        time_box_minutes: { type: 'number' },
        success_test: { type: 'string' },
        fallback: { type: 'string' },
        estimated_minutes: { type: 'number' },
        accept_overrun: { type: 'boolean' },
        feasibility: {
          type: 'object',
          additionalProperties: false,
          properties: {
            dt_s: { type: 'number' },
            cost_per_step_ms: { type: 'number' },
            inversion_conditioning: { type: 'string' },
            singular_closures: { type: 'string' },
            estimated_minutes: { type: 'number' },
            fallback: { type: 'string' },
            success_test: { type: 'string' },
          },
          required: [
            'dt_s',
            'cost_per_step_ms',
            'inversion_conditioning',
            'singular_closures',
            'estimated_minutes',
            'fallback',
            'success_test',
          ],
        },
        status: { type: 'string', enum: STEP_STATUSES },
        check: { type: 'string', enum: BASIS_CHECKS },
        symptom: { type: 'string' },
        diagnosis: { type: 'string' },
        artefact: { type: 'string' },
        signature: { type: 'string' },
        source: { type: 'string' },
        value: { type: 'number' },
        unit: { type: 'string' },
        band: {
          type: 'object',
          additionalProperties: false,
          properties: {
            plus_minus: { type: 'number' },
            low: { type: 'number' },
            high: { type: 'number' },
          },
        },
        model: { type: 'string' },
        target: { type: 'string' },
        job_id: { type: 'string' },
        label: { type: 'string' },
        dt_s: { type: 'number' },
        cost_per_step_ms: { type: 'number' },
        inversion_conditioning: { type: 'string' },
        singular_closures: { type: 'string' },
        at_risk: { type: 'array', items: { type: 'string' } },
        remaining_minutes: { type: 'number' },
        fallback_started: { type: 'boolean' },
        extend_minutes: { type: 'number' },
      },
      required: ['action'],
    },
    output: { schema: OUTPUT_SCHEMA, render: renderPlan },
    async execute(args, exec) {
      const session = exec?.agent?.session
      if (!session) throw new Error('azoth_plan: no calling session')
      const state = planFor(session)
      if (!state) throw new Error('azoth_plan: no calling session')
      const result = reduce(state, args ?? {}, { cfg, callId: exec?.callId })
      return {
        ok: true,
        action: args?.action,
        message: result.message,
        record: snapshotLine(state),
        state: serialize(state),
      }
    },
  })

  const gate = (exec) => {
    if (exec.name !== 'azoth_calc') return undefined
    const session = exec.agent?.session
    if (!session) return undefined
    try {
      return holdReason(planFor(session), cfg)
    } catch (error) {
      ctx.logger?.warn?.(`azoth-plan: cannot read state: ${String(error)}`)
      return {
        reason:
          'azoth_calc is held: the plan guard could not read its state; call azoth_plan status.',
        info: { name: 'AzothPlanState', code: 'AZOTH_PLAN_STATE' },
      }
    }
  }
  if (typeof ctx.tools.guard === 'function') {
    ctx.tools.guard(gate)
  }
  if (typeof ctx.on === 'function') {
    ctx.on('tools/pre-execute', async (exec, next) => {
      if (exec.name === 'bash') {
        const decision = bashGate(exec, cfg)
        return decision ?? next()
      }
      if (exec.name === 'azoth_calc' && typeof ctx.tools.guard !== 'function') {
        const denial = gate(exec)
        return denial ? { kind: 'deny', ...denial } : next()
      }
      return next()
    })
  }

  if (typeof ctx.on === 'function') {
    ctx.on('tools/execute', async (exec, next) => {
      const external = exec.name === 'bash' ? externalRuns.get(exec) : null
      if (!watched(exec.name) && !external) return next()
      let result
      try {
        result = await next()
      } finally {
        bodyRan.add(exec)
        if (external) {
          const state = planFor(exec.agent?.session)
          if (state) {
            const permit = findPermit(state, external.permitId)
            if (permit && permit.status === 'requested') {
              markPermitApproved(
                state,
                permit.id,
                exec.callId ?? null,
                external.mode === 'launcher' ? 'launcher' : 'user',
              )
            }
            consumePermit(state, permit, exec.callId ?? null)
            state.calcSinceReview += 1
          }
        }
        if (exec.name === 'azoth_calc') {
          const state = planFor(exec.agent?.session)
          if (state) {
            state.calcCount += 1
            state.calcSinceReview += 1
          }
        }
      }
      return result
    })

    ctx.on('tools/post-execute', async (exec, result, next) => {
      const decision = await next()
      if (exec.name === 'bash') {
        const external = externalRuns.get(exec)
        if (!external || !bodyRan.has(exec)) return decision
        const state = planFor(exec.agent?.session)
        const permit = state ? findPermit(state, external.permitId) : null
        const outcome = externalOutcome(result)
        if (state) {
          state.externalLog.push({
            callId: exec.callId ?? null,
            permit: permit?.id ?? external.permitId,
            command: external.command.slice(0, 300),
            exitCode: outcome.exitCode,
            timedOut: outcome.timedOut,
            kind: outcome.kind,
            failed: outcome.failed,
            at: now(),
            step: state.activeStepId,
          })
          if (state.externalLog.length > MAX_EXTERNAL_LOG) {
            state.externalLog = state.externalLog.slice(-MAX_EXTERNAL_LOG)
          }
          if (outcome.failed) {
            const detail = externalFailure(result)
            try {
              recordFailure(
                state,
                {
                  symptom: detail.symptom,
                  evidence: detail.evidence,
                  artefact: external.command.slice(0, 500),
                },
                { cfg, callId: exec.callId ?? null },
                'external',
              )
            } catch (error) {
              ctx.logger?.warn?.(`azoth-plan: cannot record external failure: ${String(error)}`)
            }
          }
          const jobId = result?.value?.jobId
          if (jobId && (outcome.kind === 'promoted' || outcome.kind === 'background')) {
            try {
              recordJob(state, {
                job_id: jobId,
                purpose: permit?.purpose ?? external.command,
                label: external.command.slice(0, 300),
              })
            } catch (error) {
              ctx.logger?.warn?.(`azoth-plan: cannot record job: ${String(error)}`)
            }
          }
        }
        const sections = [externalBanner(permit, external.command, outcome)]
        if (state) sections.push(snapshotLine(state))
        return prepend(decision, result ?? {}, sections.join('\n\n'))
      }
      if (!watched(exec.name) || !bodyRan.has(exec)) return decision
      const state = planFor(exec.agent?.session)
      if (!state) return decision
      let signals = []
      try {
        signals = exec.name === 'azoth_calc' ? evaluateCalc(result, cfg) : evaluateIntake(result)
      } catch (error) {
        ctx.logger?.warn?.(`azoth-plan: cannot evaluate result: ${String(error)}`)
        return decision
      }
      const calcId = result?.value?.id ?? exec.arguments?.id ?? null
      const outcome = applySignals(state, signals, {
        calcId,
        callId: exec.callId ?? null,
        cfg,
      })
      if (exec.name === 'azoth_calc') {
        state.calcLog.push({
          callId: exec.callId ?? null,
          id: calcId,
          at: now(),
          calcSeq: state.calcCount,
          isError: result?.isError === true,
          falsified: outcome.falsified,
        })
        if (state.calcLog.length > 100) state.calcLog = state.calcLog.slice(-100)
      }
      const sections = []
      if (outcome.trips.length) sections.push(tripBanner(outcome.trips))
      const checkpoint = exec.name === 'azoth_calc' ? checkpointBanner(state, cfg) : ''
      if (checkpoint) sections.push(checkpoint)
      if (!sections.length) return decision
      sections.push(snapshotLine(state))
      return prepend(decision, result ?? {}, sections.join('\n\n'))
    })

    ctx.on('session/disposed', (session) => {
      const key = stateKey(session?.session ?? session)
      if (key !== null) plans.delete(key)
    })
    ctx.on('agent/disposed', (event) => {
      const key = stateKey(event?.agent?.session ?? event?.session)
      if (key !== null) plans.delete(key)
    })
  }

  ctx.inject?.(['systemPrompt'], (scoped) => {
    let order = 510
    try {
      order = scoped.systemPrompt.getSectionOrder('PLAN_POLICY') + 10
    } catch {
      order = 510
    }
    scoped.systemPrompt.section({
      name: 'azoth:plan-guard',
      order,
      text: (assembly) => `${cfg.policy.trimEnd()}\n\n${statusFor(assembly?.agent?.session)}`,
    })
  })
}

export const internals = {
  CATEGORIES,
  FALSIFIERS,
  AUTOMATIC,
  PERMIT_SCOPES,
  normalizeConfig,
  classifyCommand,
  bashGate,
  findPermit,
  matchingPermit,
  recordPermit,
  externalOutcome,
  createState,
  createDecision,
  reduce,
  evaluateCalc,
  evaluateIntake,
  applySignals,
  falsifierMatches,
  holdReason,
  serialize,
  snapshotLine,
  renderState,
  statusFor,
  planFor,
  decisionState,
  activeDecision,
  assumptionStatus,
  openTrips,
  holdingTrips,
  chosenCandidate,
  findAssumption,
  normalizeState,
  compactState,
}
