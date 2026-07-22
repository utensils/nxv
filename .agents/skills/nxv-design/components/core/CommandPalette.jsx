import React from 'react';
import { Kbd } from './Kbd.jsx';

/**
 * CommandPalette — the ⌘K jump-to overlay: a search input over a list of
 * grouped items with keyboard hints. Render at the app root and drive `open`
 * from a ⌘K handler. Use `embedded` to render just the panel (no overlay/fixed
 * positioning) for specimens or inline docks.
 */
export function CommandPalette({
  open = false,
  embedded = false,
  onClose,
  items = [],
  placeholder = 'jump to package, run command, or search…',
  onSubmit,
  style,
  ...rest
}) {
  if (!open && !embedded) return null;
  const panel = (
    <div
      role="dialog"
      aria-modal={!embedded}
      aria-label="Command palette"
      style={{
        position: 'relative',
        width: '100%',
        maxWidth: embedded ? '100%' : 600,
        background: 'var(--surface-panel)',
        border: '1px solid var(--border-strong)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
        boxShadow: embedded ? 'none' : 'var(--shadow-pop)',
        ...style,
      }}
      {...rest}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, height: 50, padding: '0 18px', borderBottom: '1px solid var(--border-subtle)' }}>
        <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent)', fontSize: 14 }}>»</span>
        <input
          autoFocus={!embedded}
          placeholder={placeholder}
          onKeyDown={(e) => { if (e.key === 'Enter' && onSubmit) { onSubmit(e.currentTarget.value); onClose && onClose(); } }}
          style={{ flex: 1, background: 'transparent', border: 0, outline: 'none', fontFamily: 'var(--font-mono)', fontSize: 14, color: 'var(--text-heading)' }}
        />
        <Kbd>esc</Kbd>
      </div>
      <ul style={{ listStyle: 'none', margin: 0, padding: 6, maxHeight: embedded ? 'none' : '50vh', overflowY: 'auto' }}>
        {items.map((it, i) => (
          <li
            key={i}
            onClick={() => { it.onSelect && it.onSelect(); onClose && onClose(); }}
            style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px', borderRadius: 'var(--radius-sm)', cursor: 'pointer', fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--text)' }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--surface-hover)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            {it.icon && <span style={{ color: 'var(--accent)', display: 'flex' }}>{it.icon}</span>}
            <span>{it.label}</span>
            <span style={{ flex: 1 }} />
            {it.hint && <span style={{ color: 'var(--text-subtle)', fontSize: 11 }}>{it.hint}</span>}
          </li>
        ))}
      </ul>
      <div style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '8px 16px', borderTop: '1px solid var(--border-subtle)', fontFamily: 'var(--font-mono)', fontSize: 10.5, color: 'var(--text-subtle)' }}>
        <span><Kbd>↑↓</Kbd> navigate</span><span><Kbd>↵</Kbd> select</span>
      </div>
    </div>
  );
  if (embedded) return panel;
  return (
    <div style={{ position: 'fixed', inset: 0, zIndex: 70, display: 'flex', justifyContent: 'center', alignItems: 'flex-start', paddingTop: '12vh' }}>
      <div onClick={onClose} style={{ position: 'absolute', inset: 0, background: 'var(--overlay)', backdropFilter: 'var(--blur-overlay)' }} />
      {panel}
    </div>
  );
}
