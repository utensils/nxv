import * as React from 'react';

export interface PaletteItem {
  label: React.ReactNode;
  /** Right-aligned hint text (e.g. category). */
  hint?: React.ReactNode;
  icon?: React.ReactNode;
  onSelect?: () => void;
}

/**
 * CommandPalette — the ⌘K jump-to overlay (search input + grouped item list +
 * keyboard hints). Render at the app root; use `embedded` for an inline panel.
 */
export interface CommandPaletteProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Show the overlay. @default false */
  open?: boolean;
  /** Render just the panel, no fixed overlay (specimens / docks). @default false */
  embedded?: boolean;
  onClose?: () => void;
  items?: PaletteItem[];
  placeholder?: string;
  /** Called with the input value on Enter. */
  onSubmit?: (value: string) => void;
}

export function CommandPalette(props: CommandPaletteProps): JSX.Element | null;
