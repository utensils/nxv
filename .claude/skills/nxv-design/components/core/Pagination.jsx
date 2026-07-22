import React from 'react';
import { Button } from './Button.jsx';

/**
 * Pagination — the results pager: a mono "page x / y · showing a–b of n"
 * readout with prev/next ghost buttons. Compose beneath a results list.
 */
export function Pagination({ page = 1, pageSize = 50, total = 0, hasMore, onPrev, onNext, style, ...rest }) {
  const totalPages = Math.max(1, Math.ceil(total / pageSize));
  const start = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const end = Math.min(total, page * pageSize);
  const more = hasMore != null ? hasMore : page < totalPages;
  const num = { color: 'var(--text-heading)' };
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)', ...style }} {...rest}>
      <span>
        page <span style={num}>{page}</span> / <span style={num}>{totalPages}</span> · showing <span style={num}>{start}–{end}</span> of <span style={num}>{Number(total).toLocaleString()}</span>
      </span>
      <div style={{ display: 'flex', gap: 6 }}>
        <Button size="sm" variant="ghost" disabled={page <= 1} onClick={() => page > 1 && onPrev && onPrev()}>← prev</Button>
        <Button size="sm" variant="ghost" disabled={!more} onClick={() => more && onNext && onNext()}>next →</Button>
      </div>
    </div>
  );
}
