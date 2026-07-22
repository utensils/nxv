import React from 'react';

/**
 * Panel — the base surface container. Translucent "glass" over the blueprint
 * grid by default; `solid` for opaque contexts. An optional left accent rail
 * echoes the metric/card treatment.
 */
export function Panel({
  glass = true,
  rail = false,
  pad = 'md',
  radius = 'xl',
  style,
  children,
  ...rest
}) {
  const pads = { none: 0, sm: '18px', md: '24px', lg: '32px' };
  const radii = {
    lg: 'var(--radius-lg)',
    xl: 'var(--radius-xl)',
    '2xl': 'var(--radius-2xl)',
  };
  return (
    <div
      style={{
        position: 'relative',
        overflow: 'hidden',
        padding: pads[pad] ?? pads.md,
        background: glass ? 'var(--surface-glass)' : 'var(--surface-panel)',
        border: '1px solid var(--border)',
        borderRadius: radii[radius] || radii.xl,
        ...style,
      }}
      {...rest}
    >
      {rail && (
        <span
          style={{
            position: 'absolute',
            left: 0,
            top: 0,
            bottom: 0,
            width: '2px',
            background: 'var(--accent-solid)',
            opacity: 0.5,
          }}
        />
      )}
      {children}
    </div>
  );
}
