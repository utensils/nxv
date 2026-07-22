import * as React from 'react';

/** Callout — docs admonition block (tip / note / warning / danger) for the guide + API pages. */
export interface CalloutProps extends React.HTMLAttributes<HTMLDivElement> {
  /** @default "tip" */
  tone?: 'tip' | 'info' | 'warn' | 'danger';
  /** Override the mono uppercase label (defaults to the tone name). */
  title?: React.ReactNode;
}

export function Callout(props: CalloutProps): JSX.Element;
