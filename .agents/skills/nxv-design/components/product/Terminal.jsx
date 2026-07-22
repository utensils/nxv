import React from 'react';

/**
 * Terminal — a window-chrome card (traffic-light dots + title) wrapping mono
 * content. Use for command demos, the how-it-works pipeline, and self-host
 * snippets. Pass pre-formatted children (spans with token colors) as content.
 */
export function Terminal({ title = 'zsh', children, style, ...rest }) {
  return (
    <div
      style={{
        border: '1px solid var(--border)',
        borderRadius: 'var(--radius-xl)',
        overflow: 'hidden',
        background: 'var(--surface-code)',
        boxShadow: 'var(--shadow-lg)',
        ...style,
      }}
      {...rest}
    >
      <div
        style={{
          height: '44px',
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          padding: '0 18px',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ff5f56' }} />
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ffbd2e' }} />
        <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#27c93f' }} />
        <span style={{ marginLeft: 12, fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)' }}>{title}</span>
      </div>
      <pre
        style={{
          margin: 0,
          padding: '24px 26px',
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-base)',
          lineHeight: 'var(--leading-code)',
          color: 'var(--text)',
          whiteSpace: 'pre-wrap',
          overflowX: 'auto',
        }}
      >
        {children}
      </pre>
    </div>
  );
}

/** Token color spans for use inside <Terminal> content. */
Terminal.Prompt = (p) => <span style={{ color: 'var(--accent)' }} {...p} />;
Terminal.Comment = (p) => <span style={{ color: 'var(--text-muted)' }} {...p} />;
Terminal.Hash = (p) => <span style={{ color: 'var(--ok)' }} {...p} />;
Terminal.Emph = (p) => <span style={{ color: 'var(--text-heading)' }} {...p} />;
