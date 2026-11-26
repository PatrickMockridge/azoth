import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import vm from 'node:vm'

const SOURCE = readFileSync(fileURLToPath(new URL('../client.js', import.meta.url)), 'utf8')

const React = {
  createElement(type, props, ...children) {
    return { type, props: props ?? {}, children: children.flat(Infinity) }
  },
}

function load() {
  let module
  const window = { __ModuleLoader__: { load: (loaded) => (module = loaded) } }
  vm.runInNewContext(SOURCE, { window, console })
  assert.equal(module.id, 'azoth-dsh-ui')
  const slots = new Map()
  const opened = []
  const ctx = {
    slots: {
      inject(_name, register) {
        register()
      },
      register(meta, component) {
        slots.set(meta.name, { meta, component })
        return meta
      },
    },
    workspaces: {
      list: {
        getSnapshot: () => ({
          phase: 'ready',
          items: [
            { workspaceId: 'elsewhere', title: 'Default workspace' },
            { workspaceId: 'azoth-id', title: 'azoth' },
          ],
        }),
        subscribe: () => () => {},
      },
    },
    uiWorkspace: {
      openWorkspace: async (workspaceId) => opened.push(workspaceId),
    },
    effect: (effect) => effect(),
  }
  const api = module.factory(() => React)
  assert.deepEqual([...api.inject], ['slots', 'workspaces', 'uiWorkspace'])
  api.apply(ctx)
  assert.deepEqual(opened, ['azoth-id'])
  return slots
}

function text(node) {
  if (typeof node === 'string' || typeof node === 'number') return String(node)
  if (Array.isArray(node)) return node.map(text).join(' ')
  if (!node || typeof node !== 'object') return ''
  return text(node.children)
}

function buttons(node, found = []) {
  if (Array.isArray(node)) {
    node.forEach((child) => buttons(child, found))
  } else if (node && typeof node === 'object') {
    if (node.type === 'button') found.push(node)
    buttons(node.children, found)
  }
  return found
}

function links(node, found = []) {
  if (Array.isArray(node)) {
    node.forEach((child) => links(child, found))
  } else if (node && typeof node === 'object') {
    if (node.type === 'a') found.push(node)
    links(node.children, found)
  }
  return found
}

test('the UI registers a welcome card and a starter list', () => {
  const slots = load()
  assert.deepEqual([...slots.keys()], ['conversation.hero.frame', 'conversation.hero.options'])
  assert.equal(slots.get('conversation.hero.frame').meta.id, 'azoth-welcome')
  assert.equal(slots.get('conversation.hero.options').meta.id, 'azoth-starters')
})

test('the hero card gives instructions and links the user manual', () => {
  const hero = load().get('conversation.hero.frame').component
  const tree = hero({ hero: true, children: React.createElement('div', null, 'conversation') })
  const words = text(tree)
  assert.match(words, /Azoth · process engineering/)
  assert.match(words, /Ask about a pump/)
  assert.match(words, /Choose a model in the model menu/)
  assert.match(words, /Size a pump/)
  assert.match(words, /demo separator/)
  assert.match(words, /relief and fire/)
  assert.match(words, /conversation/)
  assert.ok(links(tree).some((link) => link.props.href.endsWith('/azoth/manual/')))
  assert.doesNotMatch(words, /FEED|screening|DEEPSEEK_API_KEY|Settings/)
})

test('the card renders only on the blank home', () => {
  const hero = load().get('conversation.hero.frame').component
  assert.equal(hero({ hero: false, children: 'conversation' }), 'conversation')
})

test('three starters fill and submit the composer', () => {
  const starters = load().get('conversation.hero.options').component
  const actions = { draft: '', submits: 0, setDraft(value) { this.draft = value }, submit() { this.submits += 1 } }
  const tree = starters({ inputActions: actions })
  assert.match(text(tree), /Try an azoth question/)
  const found = buttons(tree)
  assert.equal(found.length, 3)
  assert.match(text(found[0]), /Size a pump/)
  assert.match(text(found[1]), /demo separator/)
  assert.match(text(found[2]), /relief and fire/)
  found[2].props.onClick()
  assert.match(actions.draft, /relief and fire/)
  assert.equal(actions.submits, 1)
})

test('starters are inert without composer actions', () => {
  const starters = load().get('conversation.hero.options').component
  const found = buttons(starters({ inputActions: undefined }))
  assert.equal(found.length, 3)
  assert.ok(found.every((button) => button.props.disabled))
})
