import * as React from 'react';

/** Toast — transient mono copy/confirmation notice, bottom-right of the app. */
export interface ToastProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Visible + slid-in when true; faded/offset when false. @default true */
  open?: boolean;
  /** Optional leading icon. */
  icon?: React.ReactNode;
}

export function Toast(props: ToastProps): JSX.Element;
