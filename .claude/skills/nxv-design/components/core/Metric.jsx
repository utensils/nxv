import React from 'react';
import { Panel } from './Panel.jsx';

/**
 * Metric — a big mono figure with a mono uppercase label, in a railed panel.
 * The nxv index-stats row (packages / versions / commits / history) is a grid
 * of these.
 */
export function Metric({ value, label, style, ...rest }) {
  return (
    <Panel rail pad="sm" radius="lg" style={style} {...rest}>
      <div
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-3xl)',
          fontWeight: 'var(--weight-bold)',
          letterSpacing: '-0.02em',
          color: 'var(--text-heading)',
          fontFeatureSettings: "'zero' 1",
        }}
      >
        {value}
      </div>
      <div
        style={{
          marginTop: '9px',
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-2xs)',
          textTransform: 'uppercase',
          letterSpacing: 'var(--tracking-label)',
          color: 'var(--text-muted)',
        }}
      >
        {label}
      </div>
    </Panel>
  );
}
