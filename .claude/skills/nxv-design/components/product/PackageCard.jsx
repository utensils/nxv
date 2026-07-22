import React from 'react';
import { Chip } from '../core/Chip.jsx';
import { VersionBadge } from './VersionBadge.jsx';
import { Button } from '../core/Button.jsx';

const ICONS = {
  flake: <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>,
  run: <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>,
  history: <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
  shield: <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M12 8v4M12 16h.01"/></svg>,
};

const dt = { fontSize: '9px', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-subtle)' };
const dd = { margin: 0, fontFamily: 'var(--font-mono)', fontSize: 'var(--text-2xs)', color: 'var(--text-muted)' };

/**
 * PackageCard — the grid/card presentation of a search result. Same data as
 * PackageRow, laid out with a header (name/attr + version), description,
 * flag/platform chips, a first/last/rev meta strip, and three actions.
 */
export function PackageCard({ pkg, onCopyFlake, onCopyRun, onHistory, style, ...rest }) {
  const tone = pkg.insecure ? 'danger' : pkg.legacy ? 'warn' : 'brand';
  const [hover, setHover] = React.useState(false);
  return (
    <article
      tabIndex={0}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
        gap: '11px',
        padding: '16px',
        background: 'var(--surface-glass)',
        border: '1px solid',
        borderColor: hover ? 'var(--accent-hover)' : 'var(--border-subtle)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: hover ? 'var(--shadow-md)' : 'var(--shadow-sm)',
        transform: hover ? 'translateY(-2px)' : 'none',
        transition: 'var(--transition)',
        cursor: 'pointer',
        ...style,
      }}
      {...rest}
    >
      <header style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', minWidth: 0 }}>
        <div style={{ minWidth: 0 }}>
          <h3 style={{ margin: 0, fontFamily: 'var(--font-mono)', fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-semibold)', color: 'var(--text-heading)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{pkg.name}</h3>
          <p style={{ margin: '2px 0 0', fontFamily: 'var(--font-mono)', fontSize: 'var(--text-2xs)', color: 'var(--text-subtle)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            <span style={{ color: 'var(--ink-400)', marginRight: '4px' }}>›</span>{pkg.attr}
          </p>
        </div>
        <VersionBadge version={pkg.version} tone={tone} style={{ alignSelf: 'flex-start', flexShrink: 0 }} />
      </header>
      <p style={{ margin: 0, fontSize: 'var(--text-xs)', lineHeight: 'var(--leading-normal)', color: 'var(--text)', minHeight: '34px', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>{pkg.description || '—'}</p>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
        {pkg.insecure && <Chip tone="danger" size="sm" icon={ICONS.shield}>insecure</Chip>}
        {pkg.legacy && <Chip tone="warn" size="sm">pre-flakes</Chip>}
        {(pkg.platforms || []).slice(0, 3).map((p) => <Chip key={p} size="sm">{p}</Chip>)}
        {pkg.license && <Chip size="sm">{pkg.license}</Chip>}
      </div>
      <dl style={{ display: 'flex', gap: '14px', margin: '2px 0 0', paddingTop: '10px', borderTop: '1px dashed var(--border-subtle)' }}>
        <div><dt style={dt}>first</dt><dd style={dd}>{pkg.first || '—'}</dd></div>
        <div><dt style={dt}>last</dt><dd style={dd}>{pkg.last || '—'}</dd></div>
        <div><dt style={dt}>rev</dt><dd style={dd}>#{(pkg.hash || '').slice(0, 7)}</dd></div>
      </dl>
      <footer style={{ display: 'flex', gap: '6px' }} onClick={(e) => e.stopPropagation()}>
        <Button size="sm" variant="ghost" iconLeft={ICONS.flake} style={{ flex: 1 }} onClick={() => onCopyFlake && onCopyFlake(pkg)}>flake</Button>
        <Button size="sm" variant="ghost" iconLeft={ICONS.run} style={{ flex: 1 }} onClick={() => onCopyRun && onCopyRun(pkg)}>run</Button>
        <Button size="sm" variant="ghost" iconLeft={ICONS.history} style={{ flex: 1 }} onClick={() => onHistory && onHistory(pkg)}>history</Button>
      </footer>
    </article>
  );
}
