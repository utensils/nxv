import * as React from 'react';

/**
 * Panel — base surface container. Translucent glass over the blueprint grid,
 * or solid. Compose feature cards, stat panels, and modals from it.
 */
export interface PanelProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Translucent glass surface (over grid) vs opaque panel. @default true */
  glass?: boolean;
  /** Show the left Nix-blue accent rail. @default false */
  rail?: boolean;
  /** @default "md" */
  pad?: 'none' | 'sm' | 'md' | 'lg';
  /** @default "xl" */
  radius?: 'lg' | 'xl' | '2xl';
}

export function Panel(props: PanelProps): JSX.Element;
