import React from 'react';

/** Kbd — a keyboard-key cap. Used in the palette, focus hints, shortcuts. */
export function Kbd({ style, children, ...rest }) {
  return (
    <kbd
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        minWidth: '18px',
        padding: '1px 6px',
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-2xs)',
        lineHeight: 1.5,
        color: 'var(--text)',
        background: 'var(--surface-code)',
        border: '1px solid var(--border)',
        borderBottomWidth: '2px',
        borderRadius: 'var(--radius-xs)',
        ...style,
      }}
      {...rest}
    >
      {children}
    </kbd>
  );
}
