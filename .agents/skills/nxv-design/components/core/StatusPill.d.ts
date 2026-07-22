import * as React from 'react';

/** StatusPill — bordered mono pill with a glowing status dot (api health, index freshness). */
export interface StatusPillProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Dot color + glow. @default "ok" */
  tone?: 'ok' | 'warn' | 'danger' | 'idle';
}

export function StatusPill(props: StatusPillProps): JSX.Element;
