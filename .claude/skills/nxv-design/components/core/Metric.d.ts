import * as React from 'react';

/**
 * Metric — a big mono figure + uppercase label in a railed panel.
 * Lay several in a grid for the index-stats row.
 */
export interface MetricProps extends React.HTMLAttributes<HTMLDivElement> {
  /** The headline figure, e.g. "186,417" or "9+ yrs". */
  value: React.ReactNode;
  /** Lowercase caption, e.g. "packages". */
  label: React.ReactNode;
}

export function Metric(props: MetricProps): JSX.Element;
