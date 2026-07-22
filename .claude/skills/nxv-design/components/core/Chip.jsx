import React from 'react';

const TONES = {
  default: { color: 'var(--text)', border: 'var(--border)', bg: 'var(--surface-raised)' },
  active: { color: 'var(--nix-300)', border: 'var(--accent-hover)', bg: 'var(--nix-wash)' },
  ok: { color: 'var(--ok)', border: 'oklch(0.78 0.15 155 / 0.45)', bg: 'var(--green-wash)' },
  warn: { color: 'var(--warn)', border: 'oklch(0.78 0.14 80 / 0.5)', bg: 'var(--amber-wash)' },
  danger: { color: 'var(--danger)', border: 'oklch(0.66 0.19 25 / 0.55)', bg: 'var(--red-wash)' },
};

/**
 * Chip — compact mono token for filters, tags, platforms and status flags.
 * Interactive when `onClick` is provided (used for filter cycling).
 */
export function Chip({ tone = 'default', icon, size = 'md', style, children, ...rest }) {
  const t = TONES[tone] || TONES.default;
  const pad = size === 'sm' ? '2px 8px' : '4px 11px';
  const fs = size === 'sm' ? 'var(--text-2xs)' : 'var(--text-xs)';
  return (
    <span
      className="nxv-chip"
      data-tone={tone}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '5px',
        padding: pad,
        fontFamily: 'var(--font-mono)',
        fontSize: fs,
        lineHeight: 1.5,
        whiteSpace: 'nowrap',
        borderRadius: 'var(--radius-sm)',
        border: `1px solid ${t.border}`,
        background: t.bg,
        color: t.color,
        cursor: rest.onClick ? 'pointer' : 'default',
        transition: 'var(--transition)',
        ...style,
      }}
      {...rest}
    >
      {icon}
      {children}
    </span>
  );
}
