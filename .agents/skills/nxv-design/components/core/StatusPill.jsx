import React from 'react';

const TONES = {
  ok: { color: 'var(--ok)', glow: 'var(--glow-ok)' },
  warn: { color: 'var(--warn)', glow: '0 0 7px oklch(0.78 0.14 80 / 0.7)' },
  danger: { color: 'var(--danger)', glow: '0 0 7px oklch(0.66 0.19 25 / 0.7)' },
  idle: { color: 'var(--text-subtle)', glow: 'none' },
};

/**
 * StatusPill — a bordered mono pill with a glowing status dot.
 * Used in the header for "api operational · p50 34ms" and similar.
 */
export function StatusPill({ tone = 'ok', style, children, ...rest }) {
  const t = TONES[tone] || TONES.ok;
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '7px',
        padding: '7px 12px',
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-2xs)',
        color: 'var(--fog-2)',
        border: '1px solid var(--border)',
        borderRadius: 'var(--radius-sm)',
        whiteSpace: 'nowrap',
        ...style,
      }}
      {...rest}
    >
      <span
        style={{
          width: '7px',
          height: '7px',
          borderRadius: 'var(--radius-full)',
          background: t.color,
          boxShadow: t.glow,
          flex: 'none',
        }}
      />
      {children}
    </span>
  );
}
