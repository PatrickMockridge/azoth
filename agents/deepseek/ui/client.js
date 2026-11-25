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

function AzothHero({ hero, children }) {
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
        ctx.slots.inject('conversation.hero.frame', () =>
          ctx.slots.register(
            { name: 'conversation.hero.frame', id: 'azoth-welcome', order: -100 },
            AzothHero,
          ),
        )
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
