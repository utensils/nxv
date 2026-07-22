import * as React from 'react';

/** A search-result package record. */
export interface PackageRecord {
  name: string;
  /** Nix attribute path, e.g. "python27" or "python3Packages.pip". */
  attr: string;
  version: string;
  description?: string;
  license?: string;
  /** first_commit_hash (abbreviated on display). */
  hash?: string;
  /** first-seen date label. */
  first?: string;
  /** last-seen date label. */
  last?: string;
  /** platform labels, e.g. "x86_64·linux". */
  platforms?: string[];
  /** known vulnerabilities present → renders danger tone. */
  insecure?: boolean;
  /** last-seen predates the flakes epoch → renders warn tone. */
  legacy?: boolean;
}

/** PackageRow — dense scannable search-result row. Compose inside a headed list. */
export interface PackageRowProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'onCopy'> {
  pkg: PackageRecord;
  /** Copy the flake ref for this package. */
  onCopy?: (pkg: PackageRecord) => void;
  /** Open version history for this package. */
  onHistory?: (pkg: PackageRecord) => void;
}

export function PackageRow(props: PackageRowProps): JSX.Element;
