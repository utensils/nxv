import React from 'react';

/**
 * VersionTimeline — horizontal lifespan bars for a package's versions across a
 * shared time axis, with year gridlines and a dashed "flakes epoch" marker
 * (2020). Insecure versions render red. Drives the history drawer.
 */
export function VersionTimeline({ versions = [], height = 132, style, ...rest }) {
  const times = [];
  versions.forEach((v) => {
    const f = +new Date(v.first), l = +new Date(v.last);
    if (isFinite(f)) times.push(f);
    if (isFinite(l)) times.push(l);
  });
  const now = Date.now();
  const rawStart = times.length ? Math.min(...times) : now - 3.15e10;
  const rawEnd = times.length ? Math.max(...times) : now;
  const minSpan = 90 * 864e5;
  const span = Math.max(rawEnd - rawStart, minSpan);
  const pad = Math.max(14 * 864e5, span * 0.04);
  const A = rawStart - pad, B = rawEnd + pad, S = B - A;
  const x = (t) => ((t - A) / S) * 1000;
  const y0 = new Date(A).getUTCFullYear(), y1 = new Date(B).getUTCFullYear();
  const rows = versions.slice(0, 12);
  const rowH = 10, gap = 2;
  const h = Math.max(rows.length * (rowH + gap), 100);
  const flakeT = +new Date('2020-03-26T00:00:00Z');
  const years = [];
  for (let y = y0; y <= y1; y++) years.push(y);

  return (
    <div style={style} {...rest}>
      <svg viewBox={`0 0 1000 ${h}`} preserveAspectRatio="none" style={{ width: '100%', height, display: 'block' }}>
        {years.map((y) => {
          const gx = x(+new Date(Date.UTC(y, 0, 1)));
          if (gx < 0 || gx > 1000) return null;
          return <line key={y} x1={gx} x2={gx} y1={0} y2={h} stroke="var(--ink-600)" strokeDasharray="2 4" strokeWidth="1" />;
        })}
        {flakeT >= A && flakeT <= B && (
          <line x1={x(flakeT)} x2={x(flakeT)} y1={0} y2={h} stroke="var(--amber)" strokeDasharray="3 3" strokeWidth="1" opacity="0.5" />
        )}
        {rows.map((v, i) => {
          const x1 = Math.max(0, x(+new Date(v.first)));
          const x2 = Math.min(1000, x(+new Date(v.last)));
          const w = Math.max(2, x2 - x1);
          const yy = i * (rowH + gap);
          return (
            <g key={i}>
              <rect x={x1} y={yy} width={w} height={rowH} rx="1" fill={v.insecure ? 'var(--red)' : 'var(--nix-500)'} opacity={v.insecure ? 0.75 : 0.85} />
              {w > 30 && <text x={Math.min(980, x2 + 4)} y={yy + rowH - 1.5} fontFamily="var(--font-mono)" fontSize="8.5" fill="var(--text-muted)">{v.version}</text>}
            </g>
          );
        })}
      </svg>
      <div style={{ marginTop: '8px', display: 'flex', justifyContent: 'space-between', fontFamily: 'var(--font-mono)', fontSize: '10px', color: 'var(--text-subtle)', fontFeatureSettings: "'zero' 1" }}>
        {years.filter((_, i) => i % Math.max(1, Math.ceil(years.length / 10)) === 0).map((y) => <span key={y}>'{String(y).slice(2)}</span>)}
      </div>
    </div>
  );
}
