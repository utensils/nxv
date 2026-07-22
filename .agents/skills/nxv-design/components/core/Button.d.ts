import * as React from 'react';

/**
 * Button — primary action control, mono-labelled to match the nxv CLI voice.
 */
export interface ButtonProps extends React.HTMLAttributes<HTMLElement> {
  /** Visual weight. `primary` is the solid Nix-blue CTA. @default "default" */
  variant?: 'primary' | 'default' | 'ghost';
  /** @default "md" */
  size?: 'sm' | 'md' | 'lg';
  /** Optional leading icon node (16–20px SVG). */
  iconLeft?: React.ReactNode;
  /** Optional trailing icon node. */
  iconRight?: React.ReactNode;
  /** Prefix the label with a terminal prompt sigil (`$`). @default false */
  prompt?: boolean;
  /** Render as an anchor by passing href, or override the element with `as`. */
  as?: keyof JSX.IntrinsicElements;
  href?: string;
  disabled?: boolean;
}

export function Button(props: ButtonProps): JSX.Element;
