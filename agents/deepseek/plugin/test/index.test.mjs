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

test('apply registers the three azoth tools', () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  assert.deepEqual([...tools.keys()], ['azoth_catalogue', 'azoth_spec', 'azoth_calc'])
  const calc = tools.get('azoth_calc')
  assert.deepEqual(calc.parameters.required, ['id', 'inputs'])
  assert.equal(calc.parameters.additionalProperties, false)
  assert.equal(calc.output.schema.type, 'object')
  assert.equal(typeof calc.output.render, 'function')
  assert.ok(calc.timeoutMs > 0)
})

test('apply refuses a missing interpreter', () => {
  const { ctx } = harness()
  assert.throws(() => apply(ctx, {}), /config.python is required/)
})

test('the calc tool validates before spawning', async () => {
  const { ctx, tools } = harness()
  apply(ctx, { python: '/usr/bin/python3' })
  const calc = tools.get('azoth_calc')
  await assert.rejects(calc.execute({}, {}), /`id` must be a non-empty string/)
  await assert.rejects(calc.execute({ id: 'x' }, {}), /`inputs` must be a JSON object/)
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
  assert.match(text, /partially_verified/)
  assert.match(text, /spec 9c558ab44622/)
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
