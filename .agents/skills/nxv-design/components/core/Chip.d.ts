import * as React from 'react';

/**
 * Chip — compact mono token for filters, tags, platforms and status flags.
 */
export interface ChipProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Semantic tone. `active` = selected filter; ok/warn/danger for status. @default "default" */
  tone?: 'default' | 'active' | 'ok' | 'warn' | 'danger';
  /** @default "md" */
  size?: 'sm' | 'md';
  /** Optional leading icon (≤10px SVG). */
  icon?: React.ReactNode;
}

export function Chip(props: ChipProps): JSX.Element;
