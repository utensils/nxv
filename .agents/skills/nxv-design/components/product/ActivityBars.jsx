import React from 'react';

/**
 * ActivityBars — a compact request-activity sparkline (the stats panel's
 * "activity · 30m"). Zero-count buckets render as a dim baseline tick; active
 * buckets scale in green.
 */
export function ActivityBars({ data = [], height = 56, barWidth = 6, gap = 3, style, ...rest }) {
  const max = Math.max(1, ...data);
  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', gap: `${gap}px`, height, ...style }} {...rest}>
      {data.map((c, i) => {
        const h = c === 0 ? 3 : 6 + (c / max) * (height - 8);
        const idle = c === 0;
        return (
          <span
            key={i}
            title={`${c} req${c === 1 ? '' : 's'}`}
            style={{
              display: 'inline-block',
              width: barWidth,
              height: h,
              borderRadius: '1px',
              background: idle ? 'var(--ink-500)' : 'var(--ok)',
              opacity: idle ? 0.5 : 0.55 + (h / height) * 0.4,
              transition: 'height var(--dur) var(--ease)',
            }}
          />
        );
      })}
    </div>
  );
}
