import * as React from 'react';

export interface VersionSpan {
  version: string;
  /** first-seen date (ISO string or Date-parseable). */
  first: string;
  /** last-seen date. */
  last: string;
  insecure?: boolean;
}

/**
 * VersionTimeline — horizontal lifespan bars for a package's versions on a
 * shared time axis, with year gridlines and the dashed flakes-epoch (2020)
 * marker. Powers the version-history drawer.
 */
export interface VersionTimelineProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Newest-first; up to 12 rows are drawn. */
  versions: VersionSpan[];
  /** SVG height in px. @default 132 */
  height?: number;
}

export function VersionTimeline(props: VersionTimelineProps): JSX.Element;
