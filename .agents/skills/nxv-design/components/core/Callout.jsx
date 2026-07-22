import React from 'react';

const TONES = {
  tip: { color: 'var(--nix-300)', border: 'var(--accent-hover)', bg: 'var(--nix-wash)', label: 'TIP' },
  info: { color: 'var(--fog-2)', border: 'var(--border)', bg: 'var(--surface-raised)', label: 'NOTE' },
  warn: { color: 'var(--warn)', border: 'oklch(0.78 0.14 80 / 0.5)', bg: 'var(--amber-wash)', label: 'WARNING' },
  danger: { color: 'var(--danger)', border: 'oklch(0.66 0.19 25 / 0.5)', bg: 'var(--red-wash)', label: 'DANGER' },
};

/**
 * Callout — the docs admonition block (VitePress :::tip / :::warning). A left
 * accent rail, a mono uppercase label, and prose body.
 */
export function Callout({ tone = 'tip', title, style, children, ...rest }) {
  const t = TONES[tone] || TONES.tip;
  return (
    <div
      style={{
        position: 'relative',
        padding: '16px 20px 16px 22px',
        borderRadius: 'var(--radius-lg)',
        border: `1px solid ${t.border}`,
        background: t.bg,
        borderLeftWidth: '3px',
        borderLeftColor: t.color,
        ...style,
      }}
      {...rest}
    >
      <div
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-2xs)',
          letterSpacing: 'var(--tracking-label)',
          textTransform: 'uppercase',
          color: t.color,
          marginBottom: '7px',
        }}
      >
        {title || t.label}
      </div>
      <div
        style={{
          fontFamily: 'var(--font-sans)',
          fontSize: 'var(--text-base)',
          lineHeight: 'var(--leading-normal)',
          color: 'var(--text)',
        }}
      >
        {children}
      </div>
    </div>
  );
}
