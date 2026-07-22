import React from 'react';

const SIZES = {
  sm: { padding: '7px 13px', fontSize: 'var(--text-sm)', gap: '7px' },
  md: { padding: '13px 20px', fontSize: 'var(--text-base)', gap: '9px' },
  lg: { padding: '15px 26px', fontSize: 'var(--text-md)', gap: '10px' },
};

const VARIANTS = {
  primary: {
    background: 'var(--accent-solid)',
    color: 'var(--accent-on)',
    fontWeight: 'var(--weight-semibold)',
    boxShadow: 'var(--glow-accent)',
    borderColor: 'transparent',
  },
  default: {
    background: 'var(--surface-raised)',
    color: 'var(--text-strong)',
    borderColor: 'var(--border)',
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text)',
    borderColor: 'var(--border)',
  },
};

/**
 * Button — the primary action control. Mono-labelled to match the CLI voice.
 * A prompt sigil ($) can lead the label for terminal-flavoured CTAs.
 */
export function Button({
  variant = 'default',
  size = 'md',
  iconLeft,
  iconRight,
  prompt = false,
  as,
  disabled = false,
  style,
  children,
  ...rest
}) {
  const Tag = as || (rest.href ? 'a' : 'button');
  const s = SIZES[size] || SIZES.md;
  const v = VARIANTS[variant] || VARIANTS.default;
  return (
    <Tag
      className="nxv-btn"
      data-variant={variant}
      aria-disabled={disabled || undefined}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: s.gap,
        padding: s.padding,
        fontFamily: 'var(--font-mono)',
        fontSize: s.fontSize,
        lineHeight: 1,
        borderRadius: 'var(--radius-md)',
        border: '1px solid',
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.45 : 1,
        textDecoration: 'none',
        whiteSpace: 'nowrap',
        transition: 'var(--transition)',
        ...v,
        ...style,
      }}
      onMouseEnter={(e) => {
        if (disabled) return;
        if (variant === 'primary') {
          e.currentTarget.style.background = 'var(--accent-hover)';
          e.currentTarget.style.transform = 'translateY(-1px)';
        } else {
          e.currentTarget.style.borderColor = 'var(--accent-hover)';
          e.currentTarget.style.color = 'var(--text-heading)';
          if (variant === 'default') e.currentTarget.style.background = 'var(--surface-hover)';
        }
      }}
      onMouseLeave={(e) => {
        Object.assign(e.currentTarget.style, {
          background: v.background,
          color: v.color,
          borderColor: v.borderColor,
          transform: 'none',
        });
      }}
      {...rest}
    >
      {prompt && <span style={{ color: variant === 'primary' ? 'inherit' : 'var(--accent)' }}>$</span>}
      {iconLeft}
      {children}
      {iconRight}
    </Tag>
  );
}
