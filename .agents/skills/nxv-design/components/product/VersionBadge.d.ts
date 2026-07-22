import * as React from 'react';

/** VersionBadge — tabular version tag whose tone signals package status. */
export interface VersionBadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** The version string, e.g. "2.7.18". */
  version: React.ReactNode;
  /** brand=current, warn=pre-flakes, danger=insecure, plain=neutral. @default "brand" */
  tone?: 'brand' | 'warn' | 'danger' | 'plain';
}

export function VersionBadge(props: VersionBadgeProps): JSX.Element;
