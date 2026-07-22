import React from 'react';

/**
 * SegmentedToggle — a bordered mono segmented control. Drives the results
 * rows/cards view switch, and works for any small set of exclusive options.
 */
export function SegmentedToggle({ options = [], value, onChange, style, ...rest }) {
  const items = options.map((o) => (typeof o === 'string' ? { value: o, label: o } : o));
  return (
    <div
      role="tablist"
      style={{
        display: 'inline-flex',
        padding: '3px',
        gap: '2px',
        background: 'var(--surface-code)',
        border: '1px solid var(--border-subtle)',
        borderRadius: 'var(--radius-md)',
        ...style,
      }}
      {...rest}
    >
      {items.map((it) => {
        const active = it.value === value;
        return (
          <button
            key={it.value}
            role="tab"
            aria-selected={active}
            onClick={() => onChange && onChange(it.value)}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '5px 12px',
              fontFamily: 'var(--font-mono)',
              fontSize: 'var(--text-xs)',
              border: 0,
              borderRadius: 'var(--radius-xs)',
              cursor: 'pointer',
              transition: 'var(--transition)',
              background: active ? 'var(--surface-hover)' : 'transparent',
              color: active ? 'var(--text-heading)' : 'var(--text-muted)',
            }}
          >
            {it.icon}
            {it.label}
          </button>
        );
      })}
    </div>
  );
}
