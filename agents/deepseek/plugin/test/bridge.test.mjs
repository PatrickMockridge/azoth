import assert from 'node:assert/strict'
import { chmodSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import {
  buildInvocation,
  parseBridgeStdout,
  runBridge,
  runProcess,
} from '../bridge.js'

function fakeScript(body) {
  const directory = mkdtempSync(join(tmpdir(), 'azoth-bridge-'))
  const path = join(directory, 'fake.sh')
  writeFileSync(path, `#!/bin/sh\n${body}\n`)
  chmodSync(path, 0o755)
  return path
}

const CONFIG = {
  python: '/usr/bin/python3',
  pythonpath: ['/repo/python/src'],
  cwd: process.cwd(),
}

test('run puts the inputs on stdin and names the id', () => {
  const invocation = buildInvocation(
    'run',
    { id: 'hydraulics.darcy_weisbach', inputs: { f: 0.02 }, backend: 'rust' },
    CONFIG,
  )
  assert.equal(invocation.command, '/usr/bin/python3')
  assert.deepEqual(invocation.args, [
    '-B',
    '-m',
    'azoth.harness',
    'run',
    'hydraulics.darcy_weisbach',
    '--inputs-json',
    '-',
    '--backend',
    'rust',
  ])
  assert.deepEqual(JSON.parse(invocation.stdin), { f: 0.02 })
  assert.ok(invocation.env.PYTHONPATH.startsWith('/repo/python/src'))
  assert.equal(invocation.env.PYTHONDONTWRITEBYTECODE, '1')
})

test('catalogue and spec use flags, not stdin', () => {
  const listing = buildInvocation('catalogue', { query: 'darcy', limit: 5 }, CONFIG)
  assert.deepEqual(listing.args.slice(-4), ['--query', 'darcy', '--limit', '5'])
  assert.equal(listing.stdin, undefined)
  const one = buildInvocation('spec', { id: 'eos.pt_flash', full: true }, CONFIG)
  assert.deepEqual(one.args.slice(-2), ['eos.pt_flash', '--full'])
})

test('an error envelope becomes a typed engineering refusal', () => {
  assert.throws(
    () =>
      parseBridgeStdout(
        '{"kind":"error","error":{"type":"UnknownId","field":"id","message":"nope"}}',
      ),
    (error) =>
      error.type === 'UnknownId' &&
      error.field === 'id' &&
      error.detail === 'nope' &&
      /azoth_catalogue/.test(error.message),
  )
})

test('runProcess returns the parsed envelope', async () => {
  const python = fakeScript('cat >/dev/null; printf \'{"kind":"result","id":"x"}\'')
  const document = await runBridge('run', { id: 'x', inputs: {} }, { ...CONFIG, python })
  assert.deepEqual(document, { kind: 'result', id: 'x' })
})

test('runProcess refuses a bridge that writes non-JSON', async () => {
  const python = fakeScript('printf "not json"')
  await assert.rejects(
    runBridge('version', {}, { ...CONFIG, python }),
    /non-JSON/,
  )
})

test('runProcess refuses a non-zero exit with no stdout', async () => {
  const python = fakeScript('echo failed >&2; exit 3')
  await assert.rejects(
    runBridge('version', {}, { ...CONFIG, python }),
    /calculation bridge failed \(exit 3\): failed/,
  )
})

test('runProcess enforces the timeout', async () => {
  const python = fakeScript('sleep 5')
  await assert.rejects(
    runBridge('version', {}, { ...CONFIG, python, timeoutMs: 100 }),
    /took longer than 1 s/,
  )
})

test('runProcess enforces the output cap', async () => {
  const python = fakeScript('printf "0123456789abcdef"')
  await assert.rejects(
    runBridge('version', {}, { ...CONFIG, python, maxOutputBytes: 4 }),
    /too large to display/,
  )
})

test('runProcess propagates an abort', async () => {
  const python = fakeScript('sleep 5')
  const controller = new AbortController()
  const running = runProcess(
    buildInvocation('version', {}, { ...CONFIG, python }),
    { timeoutMs: 5_000, signal: controller.signal },
  )
  controller.abort()
  await assert.rejects(running, /aborted/)
})
