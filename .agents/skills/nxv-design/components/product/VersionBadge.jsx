import React from 'react';

const TONES = {
  brand: { color: 'var(--nix-300)', border: 'oklch(0.55 0.17 262 / 0.45)', bg: 'var(--nix-wash)' },
  warn: { color: 'var(--warn)', border: 'oklch(0.78 0.14 80 / 0.45)', bg: 'var(--amber-wash)' },
  danger: { color: 'var(--danger)', border: 'oklch(0.66 0.19 25 / 0.55)', bg: 'var(--red-wash)' },
  plain: { color: 'var(--text-strong)', border: 'var(--border)', bg: 'var(--surface-raised)' },
};

/**
 * VersionBadge — the tabular-nums version tag shown on package rows/cards.
 * Tone signals status: brand (current), warn (pre-flakes), danger (insecure).
 */
export function VersionBadge({ version, tone = 'brand', style, ...rest }) {
  const t = TONES[tone] || TONES.brand;
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        padding: '3px 9px',
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-xs)',
        fontFeatureSettings: "'zero' 1",
        color: t.color,
        border: `1px solid ${t.border}`,
        background: t.bg,
        borderRadius: 'var(--radius-sm)',
        whiteSpace: 'nowrap',
        ...style,
      }}
      {...rest}
    >
      {version}
    </span>
  );
}
