import * as React from 'react';

/** ActivityBars — compact request-activity sparkline for the stats panel. */
export interface ActivityBarsProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Per-bucket request counts, oldest → newest. */
  data: number[];
  /** Overall height in px. @default 56 */
  height?: number;
  /** Bar width in px. @default 6 */
  barWidth?: number;
  /** Gap between bars in px. @default 3 */
  gap?: number;
}

export function ActivityBars(props: ActivityBarsProps): JSX.Element;
