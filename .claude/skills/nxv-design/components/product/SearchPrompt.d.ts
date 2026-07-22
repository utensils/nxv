import * as React from 'react';

/**
 * SearchPrompt — nxv's signature terminal-style search bar (`nxv:~$ search …`)
 * with a blinking caret and focus-ring lift.
 */
export interface SearchPromptProps extends Omit<React.HTMLAttributes<HTMLFormElement>, 'onChange' | 'onSubmit'> {
  value?: string;
  onChange?: (value: string) => void;
  onSubmit?: (value: string) => void;
  /** @default "python 2.7" */
  placeholder?: string;
  /** The command word shown after the prompt. @default "search" */
  command?: string;
  /** Prompt hostname. @default "nxv" */
  host?: string;
  /** Show an inline "search" button instead of the blinking caret. @default false */
  button?: boolean;
}

export function SearchPrompt(props: SearchPromptProps): JSX.Element;
