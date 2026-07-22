import * as React from 'react';

export interface SegmentOption {
  value: string;
  label?: React.ReactNode;
  icon?: React.ReactNode;
}

/** SegmentedToggle — bordered mono segmented control (results view switch, sort, etc). */
export interface SegmentedToggleProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'onChange'> {
  /** Options as strings or {value,label,icon}. */
  options: (string | SegmentOption)[];
  /** Currently selected value. */
  value: string;
  /** Called with the new value on select. */
  onChange?: (value: string) => void;
}

export function SegmentedToggle(props: SegmentedToggleProps): JSX.Element;
