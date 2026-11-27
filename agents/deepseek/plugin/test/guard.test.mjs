import assert from 'node:assert/strict'
import test from 'node:test'

import { apply, internals } from '../guard.js'

let callId = 0
let sessionId = 0

function session(id = `session-${++sessionId}`, events = []) {
  return { id, snapshotEvents: () => events }
}

function harness(config = {}) {
  const tools = new Map()
  const guards = []
  const handlers = new Map()
  const sections = []
  const ctx = {
    tools: {
      register: (tool) => tools.set(tool.name, tool),
      guard: (fn) => guards.push(fn),
    },
    on: (name, fn) => {
      const list = handlers.get(name) ?? []
      list.push(fn)
      handlers.set(name, list)
      return () => {}
    },
    inject: (_services, fn) => fn(ctx),
    systemPrompt: { section: (section) => sections.push(section), getSectionOrder: () => 500 },
    logger: { warn() {} },
  }
  apply(ctx, {
    policy: 'PLAN POLICY',
    requirePlanAfter: 1,
    reviewAfterCalcs: 6,
    ...config,
  })
  return { ctx, tools, guards, handlers, sections }
}

function candidates() {
  return [
    {
      id: 'C1',
      path: 'PR flash at the separator conditions',
      cost: 'low',
      assumptions: [
        {
          id: 'A1',
          text: 'the PR flash converges for this mixture',
          category: 'numerical',
          falsifier: 'hard_warning',
          calc: 'eos.pt_flash',
        },
      ],
    },
    {
      id: 'C2',
      path: 'SRK flash with a volume translation',
      cost: 'medium',
      assumptions: [
        {
          id: 'A2',
          text: 'SRK is applicable to the polar fraction',
          category: 'applicability',
          falsifier: 'manual',
        },
      ],
    },
  ]
}

function plan(tools, s, args) {
  return tools.get('azoth_plan').execute(args, {
    agent: { session: s },
    callId: `plan-${++callId}`,
  })
}

async function openAndChoose(h, s, candidate = 'C1') {
  await plan(h.tools, s, {
    action: 'open',
    problem: 'choose a flash model',
    criteria: ['accuracy', 'cost'],
    candidates: candidates(),
  })
  return plan(h.tools, s, {
    action: 'choose',
    decision: 'D1',
    candidate,
    rationale: 'the cheapest path that still covers the mixture',
  })
}

async function calc(h, s, { id = 'eos.pt_flash', value, isError = false } = {}) {
  const exec = {
    name: 'azoth_calc',
    arguments: { id },
    callId: `calc-${++callId}`,
    agent: { session: s },
  }
  const result = isError
    ? {
        isError: true,
        error: { type: 'SolverNotConvergedError', message: 'the solver did not converge' },
        content: [{ type: 'text', text: 'Error: the solver did not converge' }],
      }
    : {
        isError: false,
        value: value ?? {
          id,
          result: { provenance: { warning_codes: [] }, warnings: [] },
        },
        content: [{ type: 'text', text: 'original result' }],
      }
  const execute = h.handlers.get('tools/execute')[0]
  await execute(exec, async () => result)
  const post = h.handlers.get('tools/post-execute')[0]
  const decision = await post(exec, result, async () => ({
    kind: 'accept',
    content: result.content,
  }))
  return { exec, result, decision }
}

async function intake(h, s, verdict = 'refused') {
  const exec = {
    name: 'azoth_intake',
    arguments: {},
    callId: `intake-${++callId}`,
    agent: { session: s },
  }
  const result = {
    isError: false,
    value: {
      kind: 'review',
      verdict,
      register: verdict === 'refused' ? [{ class: 'hard', code: 'NO_SOURCE' }] : [],
    },
    content: [{ type: 'text', text: 'intake result' }],
  }
  const execute = h.handlers.get('tools/execute')[0]
  await execute(exec, async () => result)
  const post = h.handlers.get('tools/post-execute')[0]
  const decision = await post(exec, result, async () => ({
    kind: 'accept',
    content: result.content,
  }))
  return { exec, result, decision }
}

async function runBash(h, s, command, { approve = true, result } = {}) {
  const exec = {
    name: 'bash',
    arguments: { command },
    callId: `bash-${++callId}`,
    agent: { session: s },
  }
  const pre = h.handlers.get('tools/pre-execute')?.[0]
  const gate = pre ? await pre(exec, async () => ({ kind: 'allow' })) : { kind: 'allow' }
  if (gate.kind === 'deny') return { exec, gate, result: null, decision: null }
  if (gate.kind === 'ask' && !approve) return { exec, gate, result: null, decision: null }
  const outcome = result ?? {
    isError: false,
    value: { kind: 'foreground', exitCode: 0, timedOut: false },
    content: [{ type: 'text', text: 'external output' }],
  }
  const execute = h.handlers.get('tools/execute')?.[0]
  if (execute) await execute(exec, async () => outcome)
  const post = h.handlers.get('tools/post-execute')?.[0]
  const decision = post
    ? await post(exec, outcome, async () => ({ kind: 'accept', content: outcome.content }))
    : { kind: 'accept', content: outcome.content }
  return { exec, gate, result: outcome, decision }
}

test('concept selection needs two candidates and a chosen path', async () => {
  const h = harness()
  const s = session()
  await assert.rejects(
    plan(h.tools, s, {
      action: 'open',
      problem: 'x',
      criteria: ['accuracy'],
      candidates: [candidates()[0]],
    }),
    /needs 2/,
  )
  const opened = await openAndChoose(h, s)
  assert.match(opened.message, /chose C1/)
  const state = internals.planFor(s)
  assert.equal(internals.decisionState(state, state.decisions[0]), 'chosen')
  assert.equal(state.decisions[0].choice.candidate, 'C1')
  assert.equal(state.decisions[0].choice.revision, 1)
})

test('a step needs a chosen decision', async () => {
  const h = harness()
  const s = session()
  await plan(h.tools, s, {
    action: 'open',
    problem: 'x',
    criteria: ['accuracy'],
    candidates: candidates(),
  })
  await assert.rejects(
    plan(h.tools, s, { action: 'step', description: 'run the flash' }),
    /choose a decision/,
  )
  await plan(h.tools, s, {
    action: 'choose',
    decision: 'D1',
    candidate: 'C1',
    rationale: 'best',
  })
  const step = await plan(h.tools, s, {
    action: 'step',
    description: 'run the flash',
    depends_on: ['D1', 'A1'],
  })
  assert.match(step.message, /recorded S1/)
})

test('a hard warning trips the chosen path and holds azoth_calc', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  const { decision } = await calc(h, s, {
    value: {
      id: 'eos.pt_flash',
      result: {
        warnings: [{ code: 'SOLVER_NOT_CONVERGED', message: 'no convergence' }],
        provenance: { warning_codes: ['SOLVER_NOT_CONVERGED'] },
      },
    },
  })
  const banner = decision.content.map((block) => block.text).join('\n')
  assert.match(banner, /AZOTH PLAN TRIPPED/)
  assert.match(banner, /SOLVER_NOT_CONVERGED/)
  assert.match(banner, /original result/)
  const state = internals.planFor(s)
  assert.equal(internals.decisionState(state, state.decisions[0]), 'tripped')
  assert.equal(internals.holdingTrips(state).length, 1)
  const denial = h.guards[0]({ name: 'azoth_calc', agent: { session: s } })
  assert.match(denial.reason, /decision "D1" is tripped/)
  assert.equal(denial.info.code, 'AZOTH_PLAN_HELD')
  assert.equal(h.guards[0]({ name: 'azoth_plan', agent: { session: s } }), undefined)
})

test('reopen keeps the record, refuses the falsified path and allows a new choice', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await calc(h, s, {
    value: {
      id: 'eos.pt_flash',
      result: { warnings: [], provenance: { warning_codes: ['SOLVER_NOT_CONVERGED'] } },
    },
  })
  await plan(h.tools, s, {
    action: 'reopen',
    decision: 'D1',
    reason: 'the flash does not converge',
    evidence: 'SOLVER_NOT_CONVERGED on eos.pt_flash',
  })
  const state = internals.planFor(s)
  assert.equal(internals.decisionState(state, state.decisions[0]), 'open')
  assert.equal(internals.holdingTrips(state).length, 0)
  await assert.rejects(
    plan(h.tools, s, {
      action: 'choose',
      decision: 'D1',
      candidate: 'C1',
      rationale: 'try again',
    }),
    /falsified assumption "A1"/,
  )
  await plan(h.tools, s, {
    action: 'resolve',
    assumption: 'A1',
    evidence: 'a corrected PR flash converged in the rerun',
  })
  await plan(h.tools, s, {
    action: 'choose',
    decision: 'D1',
    candidate: 'C1',
    rationale: 'the corrected flash converged',
  })
  assert.equal(state.decisions[0].choice.revision, 2)
  assert.equal(internals.decisionState(state, state.decisions[0]), 'chosen')
})

test('an error result trips the path even when no assumption is bound', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  const { decision } = await calc(h, s, {
    id: 'eos.other_flash',
    isError: true,
  })
  assert.match(decision.content.map((block) => block.text).join('\n'), /AZOTH PLAN TRIPPED/)
  const state = internals.planFor(s)
  const trips = internals.holdingTrips(state)
  assert.equal(trips.length, 1)
  assert.equal(trips[0].assumption, null)
  assert.equal(trips[0].category, 'numerical')
})

test('soft warnings do not trip a hard_warning assumption but do trip any_warning', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  const soft = await calc(h, s, {
    value: {
      id: 'eos.pt_flash',
      result: { warnings: [], provenance: { warning_codes: ['OUT_OF_VALID_RANGE'] } },
    },
  })
  assert.equal(
    soft.decision.content.map((block) => block.text).join('\n'),
    'original result',
  )
  assert.equal(internals.holdingTrips(internals.planFor(s)).length, 0)

  const candidatesAny = candidates()
  candidatesAny[0].assumptions[0].falsifier = 'any_warning'
  const s2 = session('s2')
  await plan(h.tools, s2, {
    action: 'open',
    problem: 'x',
    criteria: ['accuracy'],
    candidates: candidatesAny,
  })
  await plan(h.tools, s2, {
    action: 'choose',
    decision: 'D1',
    candidate: 'C1',
    rationale: 'best',
  })
  const { decision } = await calc(h, s2, {
    value: {
      id: 'eos.pt_flash',
      result: { warnings: [], provenance: { warning_codes: ['OUT_OF_VALID_RANGE'] } },
    },
  })
  assert.match(decision.content.map((block) => block.text).join('\n'), /AZOTH PLAN TRIPPED/)
})

test('non-finite and converged:false are numerical signals, a declared skip is not', () => {
  const cfg = internals.normalizeConfig({})
  const nonFinite = internals.evaluateCalc(
    {
      isError: false,
      value: { result: { rho: { magnitude: null, unit: 'kg/m**3' } } },
    },
    cfg,
  )
  assert.deepEqual(nonFinite.map((signal) => signal.signal), ['non_finite'])
  const skipped = internals.evaluateCalc(
    {
      isError: false,
      value: { result: { rho: { magnitude: null, unit: 'kg/m**3', skipped: true } } },
    },
    cfg,
  )
  assert.equal(skipped.length, 0)
  const converged = internals.evaluateCalc(
    { isError: false, value: { result: { iterations: 200, converged: false } } },
    cfg,
  )
  assert.deepEqual(converged.map((signal) => signal.signal), ['converged_false'])
  const failed = internals.evaluateCalc({ isError: true, error: { message: 'boom' } }, cfg)
  assert.deepEqual(failed.map((signal) => signal.signal), ['error'])
})

test('an intake refusal is an evidence signal on either tool', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  const { decision } = await intake(h, s, 'refused')
  assert.match(decision.content.map((block) => block.text).join('\n'), /AZOTH PLAN TRIPPED/)
  const trips = internals.holdingTrips(internals.planFor(s))
  assert.equal(trips.length, 1)
  assert.equal(trips[0].category, 'evidence')
  assert.equal(trips[0].signal, 'intake_refused')
})

test('an explicit trip covers a non-numerical reason, and a note does not hold', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  const noted = await plan(h.tools, s, {
    action: 'trip',
    assumption: 'A1',
    category: 'applicability',
    severity: 'note',
    evidence: 'the pressure is above the correlation basis',
  })
  assert.match(noted.message, /noted/)
  assert.equal(internals.holdingTrips(internals.planFor(s)).length, 0)
  await plan(h.tools, s, {
    action: 'reopen',
    decision: 'D1',
    reason: 'recheck the note',
    evidence: 'the note alone does not falsify the assumption',
  })
  await plan(h.tools, s, {
    action: 'choose',
    decision: 'D1',
    candidate: 'C1',
    rationale: 'the note was informational',
  })
  await plan(h.tools, s, {
    action: 'trip',
    assumption: 'A1',
    category: 'applicability',
    evidence: 'the model is outside its basis',
  })
  assert.equal(internals.holdingTrips(internals.planFor(s)).length, 1)
  assert.match(
    h.guards[0]({ name: 'azoth_calc', agent: { session: s } }).reason,
    /applicability/,
  )
})

test('a checkpoint holds calculations until review', async () => {
  const h = harness({ reviewAfterCalcs: 2 })
  const s = session()
  await openAndChoose(h, s)
  await calc(h, s)
  const second = await calc(h, s)
  assert.match(
    second.decision.content.map((block) => block.text).join('\n'),
    /AZOTH PLAN CHECKPOINT/,
  )
  const denial = h.guards[0]({ name: 'azoth_calc', agent: { session: s } })
  assert.match(denial.reason, /checkpoint/)
  assert.equal(denial.info.code, 'AZOTH_PLAN_CHECKPOINT')
  await assert.rejects(
    plan(h.tools, s, {
      action: 'review',
      decision: 'D1',
      verdict: 'confirm',
      evidence: 'the second flash agreed',
      ranking: ['C2', 'C1'],
    }),
    /ranking puts "C2" first/,
  )
  await plan(h.tools, s, {
    action: 'review',
    decision: 'D1',
    verdict: 'confirm',
    evidence: 'both flashes agree',
  })
  assert.equal(h.guards[0]({ name: 'azoth_calc', agent: { session: s } }), undefined)
})

test('review reconsider reopens the decision, and a root reopen stales a child', async () => {
  const h = harness({ reviewAfterCalcs: 1 })
  const s = session()
  await openAndChoose(h, s)
  await plan(h.tools, s, {
    action: 'add',
    parent: 'D1',
    question: 'which optional model',
    candidates: [
      {
        id: 'N1',
        path: 'no optional model',
        cost: 'low',
        assumptions: [
          {
            id: 'B1',
            text: 'the base model is enough',
            category: 'applicability',
            falsifier: 'manual',
          },
        ],
      },
      {
        id: 'N2',
        path: 'add a viscosity model',
        cost: 'high',
        assumptions: [
          {
            id: 'B2',
            text: 'the viscosity model converges',
            category: 'numerical',
            falsifier: 'hard_warning',
            calc: 'eos.viscosity',
          },
        ],
      },
    ],
  })
  await plan(h.tools, s, {
    action: 'choose',
    decision: 'D2',
    candidate: 'N2',
    rationale: 'the viscosity matters here',
  })
  await plan(h.tools, s, {
    action: 'step',
    description: 'run the viscosity model',
    depends_on: ['D2', 'B2'],
  })
  const state = internals.planFor(s)
  assert.equal(state.decisions.length, 2)
  assert.equal(state.decisions[1].parent, 'D1')
  await plan(h.tools, s, {
    action: 'review',
    decision: 'D2',
    verdict: 'reconsider',
    evidence: 'a simpler check is enough',
    category: 'budget',
  })
  assert.match(
    h.guards[0]({ name: 'azoth_calc', agent: { session: s } }).reason,
    /budget/,
  )
  await plan(h.tools, s, {
    action: 'reopen',
    decision: 'D2',
    reason: 'drop the optional model',
    evidence: 'the budget is exhausted',
  })
  assert.equal(internals.decisionState(state, state.decisions[1]), 'open')
  assert.equal(internals.decisionState(state, state.decisions[0]), 'chosen')
  await plan(h.tools, s, {
    action: 'reopen',
    decision: 'D1',
    reason: 'the concept itself is wrong',
    evidence: 'the simpler concept is cheaper',
  })
  assert.equal(internals.decisionState(state, state.decisions[1]), 'stale')
  assert.equal(state.steps[0].stale, true)
})

test('a plan is required from the second calculation', async () => {
  const h = harness()
  const s = session()
  assert.equal(h.guards[0]({ name: 'azoth_calc', agent: { session: s } }), undefined)
  await calc(h, s)
  const denial = h.guards[0]({ name: 'azoth_calc', agent: { session: s } })
  assert.match(denial.reason, /without an active plan/)
  assert.equal(denial.info.code, 'AZOTH_PLAN_REQUIRED')
  await plan(h.tools, s, {
    action: 'open',
    problem: 'x',
    criteria: ['accuracy'],
    candidates: candidates(),
  })
  assert.equal(h.guards[0]({ name: 'azoth_calc', agent: { session: s } }), undefined)
})

test('sessions do not share a plan', async () => {
  const h = harness()
  const first = session('first')
  const second = session('second')
  await openAndChoose(h, first)
  assert.equal(internals.planFor(first).problem, 'choose a flash model')
  assert.equal(internals.planFor(second).decisions.length, 0)
  await calc(h, second)
  assert.match(
    h.guards[0]({ name: 'azoth_calc', agent: { session: second } }).reason,
    /without an active plan/,
  )
})

test('close refuses a held plan until forced, and a new plan can reuse ids', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await calc(h, s, {
    value: {
      id: 'eos.pt_flash',
      result: { warnings: [], provenance: { warning_codes: ['SOLVER_NOT_CONVERGED'] } },
    },
  })
  await assert.rejects(
    plan(h.tools, s, { action: 'close', reason: 'abandon' }),
    /resolve or reopen first/,
  )
  await plan(h.tools, s, { action: 'close', reason: 'abandon', force: true })
  await plan(h.tools, s, {
    action: 'open',
    problem: 'a different problem',
    criteria: ['accuracy'],
    candidates: candidates(),
  })
  const state = internals.planFor(s)
  assert.equal(state.problem, 'a different problem')
  assert.equal(state.decisions[0].candidates[0].id, 'C1')
})

test('a plan record restores the decision tree in a new session object', async () => {
  const h = harness()
  const original = session('original')
  await openAndChoose(h, original)
  const record = internals.snapshotLine(internals.planFor(original))
  const restored = session('restored', [
    { type: 'tool/result', data: { content: [{ type: 'text', text: `prefix\n${record}\n` }] } },
  ])
  const state = internals.planFor(restored)
  assert.equal(state.problem, 'choose a flash model')
  assert.equal(state.decisions[0].choice.candidate, 'C1')
  assert.equal(internals.decisionState(state, state.decisions[0]), 'chosen')
})

test('the command classifier separates sanctioned, read-only and external work', () => {
  const classify = internals.classifyCommand
  assert.equal(classify(''), 'readonly')
  assert.equal(classify('ls -la'), 'readonly')
  assert.equal(classify('cat out/report.csv | grep liquid'), 'readonly')
  assert.equal(classify('git status --short'), 'readonly')
  assert.equal(classify('mkdir -p out && cp a.csv out/'), 'neutral')
  assert.equal(classify('git commit -m "a note"'), 'neutral')
  assert.equal(classify('azoth catalogue --query darcy'), 'sanctioned-read')
  assert.equal(classify('python -m azoth.harness catalogue --query cricondenbar'), 'sanctioned-read')
  assert.equal(
    classify('python3 -m azoth.harness run eos.pt_flash --inputs-json "{}"'),
    'sanctioned-compute',
  )
  assert.equal(classify('python3 src/thermo.py'), 'external')
  assert.equal(classify('timeout 240 python3 src/pipeline.py'), 'external')
  assert.equal(classify('nohup python3 src/study.py > out/log.txt 2>&1 &'), 'external')
  assert.equal(classify('bash -c "ls"'), 'external')
  assert.equal(classify('python3 -c "import azoth"'), 'external')
  assert.equal(classify("python3 - <<'PY'\nprint(1)\nPY"), 'external')
  assert.equal(classify("cat > out/notes.txt <<'EOF'\nhello\nEOF"), 'readonly')
  assert.equal(classify('find . -name "*.py"'), 'readonly')
  assert.equal(classify('find . -name "*.py" -delete'), 'external')
  assert.equal(classify('echo $(python3 -c "print(1)")'), 'external')
  assert.equal(classify('mkdir out && python3 src/study.py'), 'external')
})

test('the prompt section carries the policy and the live hold', async () => {
  const h = harness()
  const s = session()
  assert.equal(h.sections.length, 1)
  assert.equal(h.sections[0].name, 'azoth:plan-guard')
  assert.equal(h.sections[0].order, 510)
  assert.match(h.sections[0].text({ agent: { session: s } }), /PLAN POLICY/)
  assert.match(h.sections[0].text({ agent: { session: s } }), /No plan is open/)
  await openAndChoose(h, s)
  assert.match(h.sections[0].text({ agent: { session: s } }), /chose C1/)
  await calc(h, s, {
    value: {
      id: 'eos.pt_flash',
      result: { warnings: [], provenance: { warning_codes: ['SOLVER_NOT_CONVERGED'] } },
    },
  })
  assert.match(h.sections[0].text({ agent: { session: s } }), /PLAN HELD/)
})

test('read-only and sanctioned commands pass, a bridge run needs a plan', async () => {
  const h = harness()
  const s = session()
  assert.equal((await runBash(h, s, 'ls -la')).gate.kind, 'allow')
  assert.equal(
    (await runBash(h, s, 'python -m azoth.harness catalogue --query cricondenbar')).gate.kind,
    'allow',
  )
  const refused = await runBash(
    h,
    s,
    'python -m azoth.harness run eos.pt_flash --inputs-json "{}"',
  )
  assert.equal(refused.gate.kind, 'deny')
  assert.equal(refused.gate.info.code, 'AZOTH_PLAN_REQUIRED')
  await openAndChoose(h, s)
  const run = await runBash(h, s, 'python -m azoth.harness run eos.pt_flash --inputs-json "{}"')
  assert.equal(run.gate.kind, 'allow')
})

test('an external computation without a plan is refused', async () => {
  const h = harness()
  const s = session()
  const run = await runBash(h, s, 'python3 src/thermo.py')
  assert.equal(run.gate.kind, 'deny')
  assert.equal(run.gate.info.code, 'AZOTH_EXTERNAL_PLAN_REQUIRED')
})

test('a permit asks the user, records approval and banners the run', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await plan(h.tools, s, { action: 'step', description: 'build a custom solver' })
  const denied = await runBash(h, s, 'python3 src/pipeline.py')
  assert.equal(denied.gate.kind, 'deny')
  assert.equal(denied.gate.info.code, 'AZOTH_EXTERNAL_PERMIT_REQUIRED')
  const requested = await plan(h.tools, s, {
    action: 'permit',
    command: 'python3 src/pipeline.py',
    scope: 'step',
    purpose: 'a finite-volume solver azoth does not provide',
    verification: 'compare v and h with eos.molar_enthalpy_entropy at 17 states',
  })
  assert.match(requested.message, /requested P1/)
  const asked = await runBash(h, s, 'python3 src/pipeline.py', { approve: false })
  assert.equal(asked.gate.kind, 'ask')
  assert.match(asked.gate.displayReason.en, /finite-volume solver/)
  const state = internals.planFor(s)
  assert.equal(internals.findPermit(state, 'P1').status, 'requested')
  const run = await runBash(h, s, 'python3 src/pipeline.py')
  assert.equal(run.gate.kind, 'ask')
  assert.equal(internals.findPermit(state, 'P1').status, 'approved')
  assert.equal(internals.findPermit(state, 'P1').approvedBy, 'user')
  assert.match(run.decision.content[0].text, /EXTERNAL/)
  assert.match(run.decision.content[0].text, /P1/)
  assert.match(run.decision.content[0].text, /verification/)
  assert.equal(state.externalLog.length, 1)
  const again = await runBash(h, s, 'python3 src/pipeline.py')
  assert.equal(again.gate.kind, 'allow')
  assert.equal(state.externalLog.length, 2)
})

test('a once permit covers one call', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await plan(h.tools, s, {
    action: 'permit',
    command: 'python3 src/one.py',
    scope: 'once',
    purpose: 'one verification run',
    verification: 'compare with the azoth enthalpy at the same state',
  })
  const first = await runBash(h, s, 'python3 src/one.py')
  assert.equal(first.gate.kind, 'ask')
  const state = internals.planFor(s)
  assert.equal(internals.findPermit(state, 'P1').status, 'consumed')
  const second = await runBash(h, s, 'python3 src/one.py')
  assert.equal(second.gate.kind, 'deny')
  assert.equal(second.gate.info.code, 'AZOTH_EXTERNAL_PERMIT_REQUIRED')
})

test('the launcher mode decides whether a requested permit runs', async () => {
  const deny = harness({ externalCompute: 'deny' })
  const s = session()
  await openAndChoose(deny, s)
  await plan(deny.tools, s, {
    action: 'permit',
    command: 'python3 src/x.py',
    scope: 'session',
    purpose: 'a custom verification',
    verification: 'against the azoth result',
  })
  const refused = await runBash(deny, s, 'python3 src/x.py')
  assert.equal(refused.gate.kind, 'deny')
  assert.equal(refused.gate.info.code, 'AZOTH_EXTERNAL_APPROVAL_UNAVAILABLE')

  const allow = harness({ externalCompute: 'allow' })
  const other = session()
  await openAndChoose(allow, other)
  await plan(allow.tools, other, {
    action: 'permit',
    command: 'python3 src/x.py',
    scope: 'session',
    purpose: 'a custom verification',
    verification: 'against the azoth result',
  })
  const run = await runBash(allow, other, 'python3 src/x.py')
  assert.equal(run.gate.kind, 'allow')
  assert.equal(internals.findPermit(internals.planFor(other), 'P1').approvedBy, 'launcher')
  assert.match(run.decision.content[0].text, /approved by launcher/)
})

test('a failed external run is logged with its exit code', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await plan(h.tools, s, {
    action: 'permit',
    command: 'python3 src/x.py',
    scope: 'session',
    purpose: 'a custom verification',
    verification: 'against the azoth result',
  })
  const run = await runBash(h, s, 'python3 src/x.py', {
    result: {
      isError: false,
      value: { kind: 'foreground', exitCode: 2, timedOut: false },
      content: [{ type: 'text', text: 'failed' }],
    },
  })
  const state = internals.planFor(s)
  assert.equal(state.externalLog[0].exitCode, 2)
  assert.equal(state.externalLog[0].failed, true)
  assert.match(run.decision.content[0].text, /FAILED/)
})

test('a tripped plan refuses external computation', async () => {
  const h = harness()
  const s = session()
  await openAndChoose(h, s)
  await plan(h.tools, s, {
    action: 'trip',
    decision: 'D1',
    category: 'judgment',
    evidence: 'the model is outside its basis',
  })
  const run = await runBash(h, s, 'python3 src/x.py')
  assert.equal(run.gate.kind, 'deny')
  assert.equal(run.gate.info.code, 'AZOTH_PLAN_HELD')
})
