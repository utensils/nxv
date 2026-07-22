import * as React from 'react';

/**
 * Terminal — window-chrome card wrapping mono content (command demos, the
 * how-it-works pipeline, self-host snippets).
 *
 * Sub-components color tokens inside the content:
 * `Terminal.Prompt` (accent $), `Terminal.Comment` (muted), `Terminal.Hash`
 * (green commit), `Terminal.Emph` (bright).
 */
export interface TerminalProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Window title, e.g. a working directory or command. @default "zsh" */
  title?: React.ReactNode;
}

export function Terminal(props: TerminalProps): JSX.Element & {
  Prompt: React.FC<React.HTMLAttributes<HTMLSpanElement>>;
  Comment: React.FC<React.HTMLAttributes<HTMLSpanElement>>;
  Hash: React.FC<React.HTMLAttributes<HTMLSpanElement>>;
  Emph: React.FC<React.HTMLAttributes<HTMLSpanElement>>;
};
