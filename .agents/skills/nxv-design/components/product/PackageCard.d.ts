import * as React from 'react';
import { PackageRecord } from './PackageRow';

/**
 * PackageCard — the card/grid presentation of a search result (same data as
 * PackageRow) with a first/last/rev meta strip and flake/run/history actions.
 */
export interface PackageCardProps extends React.HTMLAttributes<HTMLElement> {
  pkg: PackageRecord;
  onCopyFlake?: (pkg: PackageRecord) => void;
  onCopyRun?: (pkg: PackageRecord) => void;
  onHistory?: (pkg: PackageRecord) => void;
}

export function PackageCard(props: PackageCardProps): JSX.Element;
