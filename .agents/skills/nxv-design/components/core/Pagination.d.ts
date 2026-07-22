import * as React from 'react';

/** Pagination — mono results pager with a range readout and prev/next buttons. */
export interface PaginationProps extends React.HTMLAttributes<HTMLDivElement> {
  /** 1-based current page. @default 1 */
  page?: number;
  /** @default 50 */
  pageSize?: number;
  /** Total result count. @default 0 */
  total?: number;
  /** Force the next button enabled/disabled (else derived from page/total). */
  hasMore?: boolean;
  onPrev?: () => void;
  onNext?: () => void;
}

export function Pagination(props: PaginationProps): JSX.Element;
