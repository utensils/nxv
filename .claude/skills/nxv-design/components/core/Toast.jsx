import React from 'react';

/**
 * Toast — the transient copy/confirmation notice. Mono, bottom-right, with a
 * small accent dot. Controlled via `open`; render at the app root.
 */
export function Toast({ open = true, icon, style, children, ...rest }) {
  return (
    <div
      role="status"
      aria-live="polite"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '9px',
        padding: '11px 16px',
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-xs)',
        color: 'var(--text-strong)',
        background: 'var(--surface-panel)',
        border: '1px solid var(--border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-pop)',
        opacity: open ? 1 : 0,
        transform: open ? 'translateY(0)' : 'translateY(1rem)',
        transition: 'opacity var(--dur-slow) var(--ease), transform var(--dur-slow) var(--ease)',
        pointerEvents: 'none',
        ...style,
      }}
      {...rest}
    >
      <span
        style={{
          width: '6px',
          height: '6px',
          borderRadius: 'var(--radius-full)',
          background: 'var(--accent)',
          flex: 'none',
        }}
      />
      {icon}
      {children}
    </div>
  );
}
