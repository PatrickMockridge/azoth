let React

const STARTERS = [
  'Size a pump for 100 m³/h of water at 8 bar discharge.',
  'Check the demo separator at its current conditions.',
  'What can azoth calculate for relief and fire cases?',
]

const CARD = {
  title: 'Azoth · process engineering',
  body:
    'Ask in plant terms: pumps, pipes, valves, separators, exchangers, relief, fire or ' +
    'HAZOP. azoth runs the calculation and shows its source and verification.',
  note:
    'FEED here means Front-End Engineering Design. The demo flowsheet is loaded; ' +
    'results are screening values, not design deliverables.',
  setup:
    'First run: save your model provider’s API key on Settings → Models, then choose ' +
    'the provider and model in the composer’s model menu.',
}

const CARD_STYLE = {
  margin: '0 auto 16px',
  maxWidth: '720px',
  padding: '16px 18px',
  border: '1px solid var(--dsw-alias-border-subtle, #d8d5cc)',
  borderRadius: '10px',
  background: 'var(--dsw-alias-bg-elevated, #fbfaf7)',
  color: 'var(--dsw-alias-fg-primary, #1f2328)',
  font: '14px/1.5 system-ui, sans-serif',
}

const MUTED_STYLE = {
  margin: '8px 0 0',
  color: 'var(--dsw-alias-fg-secondary, #5b5f66)',
  fontSize: '13px',
}

const LIST_STYLE = {
  margin: '10px 0 0',
  paddingLeft: '20px',
  color: 'var(--dsw-alias-fg-secondary, #5b5f66)',
}

const BUTTON_ROW_STYLE = {
  display: 'flex',
  flexWrap: 'wrap',
  gap: '8px',
  marginTop: '8px',
}

const BUTTON_STYLE = {
  padding: '7px 11px',
  border: '1px solid var(--dsw-alias-border-subtle, #d8d5cc)',
  borderRadius: '999px',
  background: 'var(--dsw-alias-bg-base, #ffffff)',
  color: 'var(--dsw-alias-fg-primary, #1f2328)',
  cursor: 'pointer',
  font: '13px/1.3 system-ui, sans-serif',
  textAlign: 'left',
}

const BLOCKER_STYLE = {
  position: 'absolute',
  inset: 0,
  zIndex: 20,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '4px',
  padding: '12px',
  borderRadius: '12px',
  background: 'var(--dsw-alias-bg-elevated, #fbfaf7)',
  color: 'var(--dsw-alias-fg-primary, #1f2328)',
  font: '13px/1.4 system-ui, sans-serif',
  textAlign: 'center',
  pointerEvents: 'auto',
}

function KeyBlocker() {
  return React.createElement(
    'div',
    { style: BLOCKER_STYLE, role: 'alert' },
    React.createElement('strong', null, 'API key required'),
    React.createElement(
      'span',
      null,
      'Save a key on Settings → Models to start a session.',
    ),
  )
}

function AzothHero({ hero, children, setup }) {
  if (!hero) return children
  return React.createElement(
    'div',
    { style: { display: 'contents' } },
    React.createElement(
      'section',
      { style: CARD_STYLE, 'aria-label': 'Azoth process engineering' },
      React.createElement(
        'strong',
        { style: { fontSize: '15px' } },
        CARD.title,
      ),
      React.createElement('p', { style: { margin: '6px 0 0' } }, CARD.body),
      React.createElement('p', { style: MUTED_STYLE }, CARD.note),
      React.createElement('p', { style: MUTED_STYLE }, setup || CARD.setup),
      React.createElement(
        'ul',
        { style: LIST_STYLE },
        STARTERS.map((text) => React.createElement('li', { key: text }, text)),
      ),
    ),
    children,
  )
}

function AzothStarters({ inputActions }) {
  const ready = Boolean(inputActions && inputActions.setDraft && inputActions.submit)
  return React.createElement(
    'div',
    { style: { maxWidth: '720px' } },
    React.createElement(
      'div',
      { style: { ...MUTED_STYLE, margin: '0 0 2px' } },
      'Try an azoth question',
    ),
    React.createElement(
      'div',
      { style: BUTTON_ROW_STYLE },
      STARTERS.map((text) =>
        React.createElement(
          'button',
          {
            key: text,
            type: 'button',
            disabled: !ready,
            style: BUTTON_STYLE,
            onClick: () => {
              if (!ready) return
              inputActions.setDraft(text)
              inputActions.submit()
            },
          },
          text,
        ),
      ),
    ),
  )
}

window.__ModuleLoader__.load({
  id: 'azoth-dsh-ui',
  factory(require) {
    const react = require('react')
    React = react && react.createElement ? react : react.default
    return {
      inject: ['slots'],
      apply(ctx) {
        const config = (ctx && ctx.config) || {}
        const setup = config.missingKey || config.defaultMissing
          ? 'No API key is set for ' + (config.provider || 'the selected provider') +
            '. Save one on Settings → Models, then choose the provider and model in the composer.'
          : CARD.setup
        ctx.slots.inject('conversation.hero.frame', () =>
          ctx.slots.register(
            { name: 'conversation.hero.frame', id: 'azoth-welcome', order: -100 },
            (props) => AzothHero({ ...props, setup }),
          ),
        )
        if (config.missingKey) {
          ctx.slots.inject('conversation.input.overlay', () =>
            ctx.slots.register(
              { name: 'conversation.input.overlay', id: 'azoth-key-blocker', order: -100 },
              KeyBlocker,
            ),
          )
        }
        ctx.slots.inject('conversation.hero.options', () =>
          ctx.slots.register(
            {
              name: 'conversation.hero.options',
              id: 'azoth-starters',
              order: 100,
              label: 'Azoth starters',
            },
            AzothStarters,
          ),
        )
      },
    }
  },
})
