import assert from 'node:assert/strict'
import test from 'node:test'

import { apply } from '../index.js'

function harness() {
  const tools = new Map()
  return {
    ctx: { tools: { register: (tool) => tools.set(tool.name, tool) } },
    tools,
  }
}

test('apply registers the five azoth tools', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  assert.deepEqual([...tools.keys()], [
    'azoth_help',
    'azoth_catalogue',
    'azoth_spec',
    'azoth_intake',
    'azoth_calc',
  ])
  const calc = tools.get('azoth_calc')
  assert.deepEqual(calc.parameters.required, ['id'])
  assert.equal(calc.parameters.additionalProperties, false)
  assert.equal(calc.output.schema.type, 'object')
  assert.equal(typeof calc.output.render, 'function')
  assert.ok(calc.timeoutMs > 0)
  assert.match(tools.get('azoth_catalogue').description, /plant words/)
})

test('the help tool defines the ambiguous engineering terms', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const help = tools.get('azoth_help')
  return Promise.resolve(help.execute({}, {})).then((value) => {
    assert.match(value.terms.FEED, /Front-End Engineering Design/)
    assert.match(value.terms.model, /thermodynamic/)
    assert.match(value.what, /chemical-engineering/)
    const text = help.output.render({}, value).map((block) => block.text).join('\n')
    assert.match(text, /FEED: Front-End Engineering Design/)
    assert.match(text, /azoth_catalogue finds a calculation/)
  })
})

test('apply refuses a missing interpreter', () => {
  const { ctx } = harness()
  assert.throws(() => apply(ctx, {}), /config.python is required/)
})

test('the calc tool validates before spawning', async () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  await assert.rejects(calc.execute({}, {}), /`id` must be a calculation id/)
  await assert.rejects(calc.execute({ id: 'x' }, {}), /`inputs` must be an object/)
})

test('a dossier replaces inputs, and the strict mode requires one', async () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  await assert.rejects(
    calc.execute({ id: 'x', dossier: {}, inputs: {} }, {}),
    /a dossier replaces `inputs`/,
  )

  const strict = harness()
  apply(strict.ctx, { python: '/usr/bin/python3', requireDossier: true })
  await assert.rejects(
    strict.tools.get('azoth_calc').execute({ id: 'x' }, {}),
    /requires an intake dossier/,
  )
})

test('the intake tool requires a dossier and renders its register', async () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const intake = tools.get('azoth_intake')
  await assert.rejects(intake.execute({}, {}), /`dossier` or `dossier_path` is required/)
  const text = intake.output
    .render(
      {},
      {
        dossier: 'smoke',
        verdict: 'refused',
        register: [
          {
            gap: 'G-01',
            class: 'hard',
            code: 'CONFLICT_UNRESOLVED',
            input: 'P',
            detail: 'two pressures',
          },
        ],
        requests: [{ request: 'R-01', blocking: true, ask: 'reconcile P' }],
      },
    )
    .map((block) => block.text)
    .join('\n')
  assert.match(text, /intake review: smoke/)
  assert.match(text, /verdict: refused \(1 hard, 0 soft\)/)
  assert.match(text, /G-01 hard CONFLICT_UNRESOLVED P: two pressures/)
  assert.match(text, /R-01 \(blocking\) reconcile P/)
})

test('the result renderer names the fields and the provenance', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  const blocks = calc.output.render(
    {},
    {
      kind: 'result',
      id: 'hydraulics.reynolds_number',
      backend: 'rust',
      result: {
        re: 149401.2,
        regime: 'turbulent',
        warnings: [],
        provenance: {
          verification: 'partially_verified',
          spec_sha256: '9c558ab4462221e20e3d93fbe5419282ce66d6a34499aaf6b2128845fed2c734',
          rust_sha256: '4e7fb5bae8f374eaf9195a0c6267a1c6225b37ba801896ff1ae98e54fc86ff74',
        },
      },
    },
  )
  const text = blocks.map((block) => block.text).join('\n')
  assert.match(text, /hydraulics\.reynolds_number \(rust\)/)
  assert.match(text, /re = 149401\.2/)
  assert.match(text, /warnings: 0/)
  assert.match(text, /verification: partially_verified/)
  assert.match(text, /screening value, not a design deliverable/)
  assert.match(text, /spec 9c558ab44622/)
})

test('the result renderer shows the sibling intake block', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  const text = calc.output
    .render(
      {},
      {
        kind: 'result',
        id: 'eos.pt_flash',
        backend: 'python',
        result: { phase: 'two_phase', warnings: [], provenance: {} },
        intake: {
          target: 'eos.pt_flash',
          enforcement: 'advisory',
          verdict: 'passed_with_flags',
          register: [
            {
              gap: 'G-01',
              class: 'soft',
              code: 'INFERRED_COMPOSITION_REMAINDER',
              input: 'z',
              detail: 'n-heptane inferred',
            },
          ],
          requests: [
            { request: 'R-01', blocking: false, ask: 'confirm n-heptane' },
          ],
        },
      },
    )
    .map((block) => block.text)
    .join('\n')
  assert.match(text, /intake: advisory for eos\.pt_flash/)
  assert.match(text, /verdict: passed_with_flags \(0 hard, 1 soft\)/)
  assert.match(text, /R-01 confirm n-heptane/)
})

test('a result larger than the render cap is truncated', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  const blocks = calc.output.render(
    {},
    { kind: 'result', id: 'x', backend: 'rust', result: { big: 'x'.repeat(100_000) } },
  )
  assert.match(blocks[0].text, /truncated/)
})
