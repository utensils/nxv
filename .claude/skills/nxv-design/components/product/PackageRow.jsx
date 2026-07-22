import React from 'react';
import { Chip } from '../core/Chip.jsx';
import { VersionBadge } from './VersionBadge.jsx';

const ICONS = {
  copy: <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>,
  history: <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
  shield: <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M12 8v4M12 16h.01"/></svg>,
};

const actionBtn = {
  display: 'inline-flex', alignItems: 'center', gap: '6px', padding: '6px 10px',
  fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)',
  background: 'transparent', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-sm)',
  cursor: 'pointer', transition: 'var(--transition)',
};

/**
 * PackageRow — a dense, scannable search-result row: package · attr, version,
 * description with platform/flag chips, first/last dates, and copy/history
 * actions. Built on Chip + VersionBadge. Meant for a headed list container.
 */
export function PackageRow({ pkg, onCopy, onHistory, style, ...rest }) {
  const tone = pkg.insecure ? 'danger' : pkg.legacy ? 'warn' : 'brand';
  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: 'minmax(180px,1.6fr) 100px minmax(200px,2fr) 120px 90px',
        gap: '16px',
        alignItems: 'center',
        padding: '14px 20px',
        borderBottom: '1px solid var(--border-subtle)',
        cursor: 'pointer',
        transition: 'background var(--dur) var(--ease)',
        ...style,
      }}
      onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--surface-raised)')}
      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
      onClick={() => onHistory && onHistory(pkg)}
      {...rest}
    >
      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)', color: 'var(--text-heading)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{pkg.name}</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-2xs)', color: 'var(--accent)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{pkg.attr}</span>
        </div>
        <div style={{ marginTop: '3px', fontFamily: 'var(--font-mono)', fontSize: 'var(--text-2xs)', color: 'var(--text-subtle)', display: 'flex', gap: '8px' }}>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '180px' }}>{pkg.license || '—'}</span>
          <span>·</span>
          <span>#{(pkg.hash || '').slice(0, 7)}</span>
        </div>
      </div>
      <VersionBadge version={pkg.version} tone={tone} />
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--text)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{pkg.description || '—'}</div>
        <div style={{ marginTop: '6px', display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
          {pkg.insecure && <Chip tone="danger" size="sm" icon={ICONS.shield}>insecure</Chip>}
          {pkg.legacy && <Chip tone="warn" size="sm">pre-flakes</Chip>}
          {(pkg.platforms || []).slice(0, 3).map((p) => <Chip key={p} size="sm">{p}</Chip>)}
        </div>
      </div>
      <div style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)', fontFeatureSettings: "'zero' 1" }}>{pkg.first || '—'}</div>
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '6px' }} onClick={(e) => e.stopPropagation()}>
        <button style={actionBtn} title="copy flake ref" onClick={() => onCopy && onCopy(pkg)}>{ICONS.copy}</button>
        <button style={actionBtn} title="version history" onClick={() => onHistory && onHistory(pkg)}>{ICONS.history}</button>
      </div>
    </div>
  );
}
