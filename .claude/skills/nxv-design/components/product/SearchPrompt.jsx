import React from 'react';

/**
 * SearchPrompt — nxv's signature terminal-style search bar. Renders a prompt
 * sigil, the command word, an input, and a blinking caret; the whole shell
 * lifts with a focus ring on focus. Optionally shows an inline run button.
 */
export function SearchPrompt({
  value = '',
  onChange,
  onSubmit,
  placeholder = 'python 2.7',
  command = 'search',
  host = 'nxv',
  button = false,
  style,
  ...rest
}) {
  const [focused, setFocused] = React.useState(false);
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit && onSubmit(value);
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        height: '58px',
        padding: button ? '0 8px 0 20px' : '0 20px',
        background: 'var(--surface-raised)',
        border: '1px solid var(--border)',
        borderColor: focused ? 'var(--accent-hover)' : 'var(--border)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: focused ? 'var(--ring-focus)' : 'none',
        transition: 'var(--transition)',
        ...style,
      }}
      {...rest}
    >
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-md)', color: 'var(--accent)', userSelect: 'none' }}>
        {host}&nbsp;<span style={{ color: 'var(--text-subtle)' }}>:~</span>$
      </span>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-md)', color: 'var(--text-muted)', userSelect: 'none' }}>
        {command}
      </span>
      <input
        type="search"
        value={value}
        onChange={(e) => onChange && onChange(e.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        placeholder={placeholder}
        aria-label="Search nixpkgs by package and version"
        style={{
          flex: 1,
          minWidth: 0,
          background: 'transparent',
          border: 0,
          outline: 'none',
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-md)',
          color: 'var(--text-heading)',
        }}
      />
      {!button && (
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-lg)', color: 'var(--accent)', animation: 'nxv-caret 1.1s steps(1) infinite' }}>▌</span>
      )}
      {button && (
        <button
          type="submit"
          style={{
            height: '42px',
            padding: '0 20px',
            border: 0,
            borderRadius: 'var(--radius-md)',
            background: 'var(--accent-solid)',
            color: 'var(--accent-on)',
            fontFamily: 'var(--font-mono)',
            fontSize: 'var(--text-base)',
            fontWeight: 'var(--weight-semibold)',
            cursor: 'pointer',
          }}
        >
          search
        </button>
      )}
      <style>{`@keyframes nxv-caret{0%,49%{opacity:1}50%,100%{opacity:0}}`}</style>
    </form>
  );
}
