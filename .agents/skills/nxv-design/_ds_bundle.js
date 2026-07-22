/* @ds-bundle: {"format":4,"namespace":"NxvDesignSystem_0fa7ce","components":[{"name":"Button","sourcePath":"components/core/Button.jsx"},{"name":"Callout","sourcePath":"components/core/Callout.jsx"},{"name":"Chip","sourcePath":"components/core/Chip.jsx"},{"name":"CommandPalette","sourcePath":"components/core/CommandPalette.jsx"},{"name":"Kbd","sourcePath":"components/core/Kbd.jsx"},{"name":"Metric","sourcePath":"components/core/Metric.jsx"},{"name":"Pagination","sourcePath":"components/core/Pagination.jsx"},{"name":"Panel","sourcePath":"components/core/Panel.jsx"},{"name":"SegmentedToggle","sourcePath":"components/core/SegmentedToggle.jsx"},{"name":"StatusPill","sourcePath":"components/core/StatusPill.jsx"},{"name":"Toast","sourcePath":"components/core/Toast.jsx"},{"name":"ActivityBars","sourcePath":"components/product/ActivityBars.jsx"},{"name":"PackageCard","sourcePath":"components/product/PackageCard.jsx"},{"name":"PackageRow","sourcePath":"components/product/PackageRow.jsx"},{"name":"SearchPrompt","sourcePath":"components/product/SearchPrompt.jsx"},{"name":"Terminal","sourcePath":"components/product/Terminal.jsx"},{"name":"VersionBadge","sourcePath":"components/product/VersionBadge.jsx"},{"name":"VersionTimeline","sourcePath":"components/product/VersionTimeline.jsx"}],"sourceHashes":{"components/core/Button.jsx":"bc34623f1b97","components/core/Callout.jsx":"4bf02e9b383b","components/core/Chip.jsx":"4faad60f5fbe","components/core/CommandPalette.jsx":"79cfda393a58","components/core/Kbd.jsx":"e3556b1c76a1","components/core/Metric.jsx":"c21bbb0585ba","components/core/Pagination.jsx":"2fabb209432e","components/core/Panel.jsx":"609fc94ba408","components/core/SegmentedToggle.jsx":"9fedad279f0f","components/core/StatusPill.jsx":"d9d5c83d96e9","components/core/Toast.jsx":"dd66f8c2c619","components/product/ActivityBars.jsx":"09006a4200d6","components/product/PackageCard.jsx":"a69ef1bc193f","components/product/PackageRow.jsx":"631db3635f58","components/product/SearchPrompt.jsx":"9ae9bd75b70a","components/product/Terminal.jsx":"0facf7e9443b","components/product/VersionBadge.jsx":"88c1105060ad","components/product/VersionTimeline.jsx":"84714f84e4af"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.NxvDesignSystem_0fa7ce = window.NxvDesignSystem_0fa7ce || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/core/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    padding: '7px 13px',
    fontSize: 'var(--text-sm)',
    gap: '7px'
  },
  md: {
    padding: '13px 20px',
    fontSize: 'var(--text-base)',
    gap: '9px'
  },
  lg: {
    padding: '15px 26px',
    fontSize: 'var(--text-md)',
    gap: '10px'
  }
};
const VARIANTS = {
  primary: {
    background: 'var(--accent-solid)',
    color: 'var(--accent-on)',
    fontWeight: 'var(--weight-semibold)',
    boxShadow: 'var(--glow-accent)',
    borderColor: 'transparent'
  },
  default: {
    background: 'var(--surface-raised)',
    color: 'var(--text-strong)',
    borderColor: 'var(--border)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text)',
    borderColor: 'var(--border)'
  }
};

/**
 * Button — the primary action control. Mono-labelled to match the CLI voice.
 * A prompt sigil ($) can lead the label for terminal-flavoured CTAs.
 */
function Button({
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
  return /*#__PURE__*/React.createElement(Tag, _extends({
    className: "nxv-btn",
    "data-variant": variant,
    "aria-disabled": disabled || undefined,
    style: {
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
      ...style
    },
    onMouseEnter: e => {
      if (disabled) return;
      if (variant === 'primary') {
        e.currentTarget.style.background = 'var(--accent-hover)';
        e.currentTarget.style.transform = 'translateY(-1px)';
      } else {
        e.currentTarget.style.borderColor = 'var(--accent-hover)';
        e.currentTarget.style.color = 'var(--text-heading)';
        if (variant === 'default') e.currentTarget.style.background = 'var(--surface-hover)';
      }
    },
    onMouseLeave: e => {
      Object.assign(e.currentTarget.style, {
        background: v.background,
        color: v.color,
        borderColor: v.borderColor,
        transform: 'none'
      });
    }
  }, rest), prompt && /*#__PURE__*/React.createElement("span", {
    style: {
      color: variant === 'primary' ? 'inherit' : 'var(--accent)'
    }
  }, "$"), iconLeft, children, iconRight);
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Button.jsx", error: String((e && e.message) || e) }); }

// components/core/Callout.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const TONES = {
  tip: {
    color: 'var(--nix-300)',
    border: 'var(--accent-hover)',
    bg: 'var(--nix-wash)',
    label: 'TIP'
  },
  info: {
    color: 'var(--fog-2)',
    border: 'var(--border)',
    bg: 'var(--surface-raised)',
    label: 'NOTE'
  },
  warn: {
    color: 'var(--warn)',
    border: 'oklch(0.78 0.14 80 / 0.5)',
    bg: 'var(--amber-wash)',
    label: 'WARNING'
  },
  danger: {
    color: 'var(--danger)',
    border: 'oklch(0.66 0.19 25 / 0.5)',
    bg: 'var(--red-wash)',
    label: 'DANGER'
  }
};

/**
 * Callout — the docs admonition block (VitePress :::tip / :::warning). A left
 * accent rail, a mono uppercase label, and prose body.
 */
function Callout({
  tone = 'tip',
  title,
  style,
  children,
  ...rest
}) {
  const t = TONES[tone] || TONES.tip;
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      position: 'relative',
      padding: '16px 20px 16px 22px',
      borderRadius: 'var(--radius-lg)',
      border: `1px solid ${t.border}`,
      background: t.bg,
      borderLeftWidth: '3px',
      borderLeftColor: t.color,
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      letterSpacing: 'var(--tracking-label)',
      textTransform: 'uppercase',
      color: t.color,
      marginBottom: '7px'
    }
  }, title || t.label), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 'var(--text-base)',
      lineHeight: 'var(--leading-normal)',
      color: 'var(--text)'
    }
  }, children));
}
Object.assign(__ds_scope, { Callout });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Callout.jsx", error: String((e && e.message) || e) }); }

// components/core/Chip.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const TONES = {
  default: {
    color: 'var(--text)',
    border: 'var(--border)',
    bg: 'var(--surface-raised)'
  },
  active: {
    color: 'var(--nix-300)',
    border: 'var(--accent-hover)',
    bg: 'var(--nix-wash)'
  },
  ok: {
    color: 'var(--ok)',
    border: 'oklch(0.78 0.15 155 / 0.45)',
    bg: 'var(--green-wash)'
  },
  warn: {
    color: 'var(--warn)',
    border: 'oklch(0.78 0.14 80 / 0.5)',
    bg: 'var(--amber-wash)'
  },
  danger: {
    color: 'var(--danger)',
    border: 'oklch(0.66 0.19 25 / 0.55)',
    bg: 'var(--red-wash)'
  }
};

/**
 * Chip — compact mono token for filters, tags, platforms and status flags.
 * Interactive when `onClick` is provided (used for filter cycling).
 */
function Chip({
  tone = 'default',
  icon,
  size = 'md',
  style,
  children,
  ...rest
}) {
  const t = TONES[tone] || TONES.default;
  const pad = size === 'sm' ? '2px 8px' : '4px 11px';
  const fs = size === 'sm' ? 'var(--text-2xs)' : 'var(--text-xs)';
  return /*#__PURE__*/React.createElement("span", _extends({
    className: "nxv-chip",
    "data-tone": tone,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      padding: pad,
      fontFamily: 'var(--font-mono)',
      fontSize: fs,
      lineHeight: 1.5,
      whiteSpace: 'nowrap',
      borderRadius: 'var(--radius-sm)',
      border: `1px solid ${t.border}`,
      background: t.bg,
      color: t.color,
      cursor: rest.onClick ? 'pointer' : 'default',
      transition: 'var(--transition)',
      ...style
    }
  }, rest), icon, children);
}
Object.assign(__ds_scope, { Chip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Chip.jsx", error: String((e && e.message) || e) }); }

// components/core/Kbd.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Kbd — a keyboard-key cap. Used in the palette, focus hints, shortcuts. */
function Kbd({
  style,
  children,
  ...rest
}) {
  return /*#__PURE__*/React.createElement("kbd", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      minWidth: '18px',
      padding: '1px 6px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      lineHeight: 1.5,
      color: 'var(--text)',
      background: 'var(--surface-code)',
      border: '1px solid var(--border)',
      borderBottomWidth: '2px',
      borderRadius: 'var(--radius-xs)',
      ...style
    }
  }, rest), children);
}
Object.assign(__ds_scope, { Kbd });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Kbd.jsx", error: String((e && e.message) || e) }); }

// components/core/CommandPalette.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * CommandPalette — the ⌘K jump-to overlay: a search input over a list of
 * grouped items with keyboard hints. Render at the app root and drive `open`
 * from a ⌘K handler. Use `embedded` to render just the panel (no overlay/fixed
 * positioning) for specimens or inline docks.
 */
function CommandPalette({
  open = false,
  embedded = false,
  onClose,
  items = [],
  placeholder = 'jump to package, run command, or search…',
  onSubmit,
  style,
  ...rest
}) {
  if (!open && !embedded) return null;
  const panel = /*#__PURE__*/React.createElement("div", _extends({
    role: "dialog",
    "aria-modal": !embedded,
    "aria-label": "Command palette",
    style: {
      position: 'relative',
      width: '100%',
      maxWidth: embedded ? '100%' : 600,
      background: 'var(--surface-panel)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--radius-lg)',
      overflow: 'hidden',
      boxShadow: embedded ? 'none' : 'var(--shadow-pop)',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      height: 50,
      padding: '0 18px',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      color: 'var(--accent)',
      fontSize: 14
    }
  }, "\xBB"), /*#__PURE__*/React.createElement("input", {
    autoFocus: !embedded,
    placeholder: placeholder,
    onKeyDown: e => {
      if (e.key === 'Enter' && onSubmit) {
        onSubmit(e.currentTarget.value);
        onClose && onClose();
      }
    },
    style: {
      flex: 1,
      background: 'transparent',
      border: 0,
      outline: 'none',
      fontFamily: 'var(--font-mono)',
      fontSize: 14,
      color: 'var(--text-heading)'
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.Kbd, null, "esc")), /*#__PURE__*/React.createElement("ul", {
    style: {
      listStyle: 'none',
      margin: 0,
      padding: 6,
      maxHeight: embedded ? 'none' : '50vh',
      overflowY: 'auto'
    }
  }, items.map((it, i) => /*#__PURE__*/React.createElement("li", {
    key: i,
    onClick: () => {
      it.onSelect && it.onSelect();
      onClose && onClose();
    },
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '10px 12px',
      borderRadius: 'var(--radius-sm)',
      cursor: 'pointer',
      fontFamily: 'var(--font-mono)',
      fontSize: 13,
      color: 'var(--text)'
    },
    onMouseEnter: e => e.currentTarget.style.background = 'var(--surface-hover)',
    onMouseLeave: e => e.currentTarget.style.background = 'transparent'
  }, it.icon && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--accent)',
      display: 'flex'
    }
  }, it.icon), /*#__PURE__*/React.createElement("span", null, it.label), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), it.hint && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-subtle)',
      fontSize: 11
    }
  }, it.hint)))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 14,
      padding: '8px 16px',
      borderTop: '1px solid var(--border-subtle)',
      fontFamily: 'var(--font-mono)',
      fontSize: 10.5,
      color: 'var(--text-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement(__ds_scope.Kbd, null, "\u2191\u2193"), " navigate"), /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement(__ds_scope.Kbd, null, "\u21B5"), " select")));
  if (embedded) return panel;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'fixed',
      inset: 0,
      zIndex: 70,
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'flex-start',
      paddingTop: '12vh'
    }
  }, /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'absolute',
      inset: 0,
      background: 'var(--overlay)',
      backdropFilter: 'var(--blur-overlay)'
    }
  }), panel);
}
Object.assign(__ds_scope, { CommandPalette });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/CommandPalette.jsx", error: String((e && e.message) || e) }); }

// components/core/Pagination.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Pagination — the results pager: a mono "page x / y · showing a–b of n"
 * readout with prev/next ghost buttons. Compose beneath a results list.
 */
function Pagination({
  page = 1,
  pageSize = 50,
  total = 0,
  hasMore,
  onPrev,
  onNext,
  style,
  ...rest
}) {
  const totalPages = Math.max(1, Math.ceil(total / pageSize));
  const start = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const end = Math.min(total, page * pageSize);
  const more = hasMore != null ? hasMore : page < totalPages;
  const num = {
    color: 'var(--text-heading)'
  };
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", null, "page ", /*#__PURE__*/React.createElement("span", {
    style: num
  }, page), " / ", /*#__PURE__*/React.createElement("span", {
    style: num
  }, totalPages), " \xB7 showing ", /*#__PURE__*/React.createElement("span", {
    style: num
  }, start, "\u2013", end), " of ", /*#__PURE__*/React.createElement("span", {
    style: num
  }, Number(total).toLocaleString())), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    disabled: page <= 1,
    onClick: () => page > 1 && onPrev && onPrev()
  }, "\u2190 prev"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    disabled: !more,
    onClick: () => more && onNext && onNext()
  }, "next \u2192")));
}
Object.assign(__ds_scope, { Pagination });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Pagination.jsx", error: String((e && e.message) || e) }); }

// components/core/Panel.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Panel — the base surface container. Translucent "glass" over the blueprint
 * grid by default; `solid` for opaque contexts. An optional left accent rail
 * echoes the metric/card treatment.
 */
function Panel({
  glass = true,
  rail = false,
  pad = 'md',
  radius = 'xl',
  style,
  children,
  ...rest
}) {
  const pads = {
    none: 0,
    sm: '18px',
    md: '24px',
    lg: '32px'
  };
  const radii = {
    lg: 'var(--radius-lg)',
    xl: 'var(--radius-xl)',
    '2xl': 'var(--radius-2xl)'
  };
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      position: 'relative',
      overflow: 'hidden',
      padding: pads[pad] ?? pads.md,
      background: glass ? 'var(--surface-glass)' : 'var(--surface-panel)',
      border: '1px solid var(--border)',
      borderRadius: radii[radius] || radii.xl,
      ...style
    }
  }, rest), rail && /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 0,
      top: 0,
      bottom: 0,
      width: '2px',
      background: 'var(--accent-solid)',
      opacity: 0.5
    }
  }), children);
}
Object.assign(__ds_scope, { Panel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Panel.jsx", error: String((e && e.message) || e) }); }

// components/core/Metric.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Metric — a big mono figure with a mono uppercase label, in a railed panel.
 * The nxv index-stats row (packages / versions / commits / history) is a grid
 * of these.
 */
function Metric({
  value,
  label,
  style,
  ...rest
}) {
  return /*#__PURE__*/React.createElement(__ds_scope.Panel, _extends({
    rail: true,
    pad: "sm",
    radius: "lg",
    style: style
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-3xl)',
      fontWeight: 'var(--weight-bold)',
      letterSpacing: '-0.02em',
      color: 'var(--text-heading)',
      fontFeatureSettings: "'zero' 1"
    }
  }, value), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: '9px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      textTransform: 'uppercase',
      letterSpacing: 'var(--tracking-label)',
      color: 'var(--text-muted)'
    }
  }, label));
}
Object.assign(__ds_scope, { Metric });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Metric.jsx", error: String((e && e.message) || e) }); }

// components/core/SegmentedToggle.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * SegmentedToggle — a bordered mono segmented control. Drives the results
 * rows/cards view switch, and works for any small set of exclusive options.
 */
function SegmentedToggle({
  options = [],
  value,
  onChange,
  style,
  ...rest
}) {
  const items = options.map(o => typeof o === 'string' ? {
    value: o,
    label: o
  } : o);
  return /*#__PURE__*/React.createElement("div", _extends({
    role: "tablist",
    style: {
      display: 'inline-flex',
      padding: '3px',
      gap: '2px',
      background: 'var(--surface-code)',
      border: '1px solid var(--border-subtle)',
      borderRadius: 'var(--radius-md)',
      ...style
    }
  }, rest), items.map(it => {
    const active = it.value === value;
    return /*#__PURE__*/React.createElement("button", {
      key: it.value,
      role: "tab",
      "aria-selected": active,
      onClick: () => onChange && onChange(it.value),
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: '6px',
        padding: '5px 12px',
        fontFamily: 'var(--font-mono)',
        fontSize: 'var(--text-xs)',
        border: 0,
        borderRadius: 'var(--radius-xs)',
        cursor: 'pointer',
        transition: 'var(--transition)',
        background: active ? 'var(--surface-hover)' : 'transparent',
        color: active ? 'var(--text-heading)' : 'var(--text-muted)'
      }
    }, it.icon, it.label);
  }));
}
Object.assign(__ds_scope, { SegmentedToggle });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/SegmentedToggle.jsx", error: String((e && e.message) || e) }); }

// components/core/StatusPill.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const TONES = {
  ok: {
    color: 'var(--ok)',
    glow: 'var(--glow-ok)'
  },
  warn: {
    color: 'var(--warn)',
    glow: '0 0 7px oklch(0.78 0.14 80 / 0.7)'
  },
  danger: {
    color: 'var(--danger)',
    glow: '0 0 7px oklch(0.66 0.19 25 / 0.7)'
  },
  idle: {
    color: 'var(--text-subtle)',
    glow: 'none'
  }
};

/**
 * StatusPill — a bordered mono pill with a glowing status dot.
 * Used in the header for "api operational · p50 34ms" and similar.
 */
function StatusPill({
  tone = 'ok',
  style,
  children,
  ...rest
}) {
  const t = TONES[tone] || TONES.ok;
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '7px',
      padding: '7px 12px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      color: 'var(--fog-2)',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-sm)',
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    style: {
      width: '7px',
      height: '7px',
      borderRadius: 'var(--radius-full)',
      background: t.color,
      boxShadow: t.glow,
      flex: 'none'
    }
  }), children);
}
Object.assign(__ds_scope, { StatusPill });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/StatusPill.jsx", error: String((e && e.message) || e) }); }

// components/core/Toast.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Toast — the transient copy/confirmation notice. Mono, bottom-right, with a
 * small accent dot. Controlled via `open`; render at the app root.
 */
function Toast({
  open = true,
  icon,
  style,
  children,
  ...rest
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    role: "status",
    "aria-live": "polite",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '9px',
      padding: '11px 16px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-strong)',
      background: 'var(--surface-panel)',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)',
      boxShadow: 'var(--shadow-pop)',
      opacity: open ? 1 : 0,
      transform: open ? 'translateY(0)' : 'translateY(1rem)',
      transition: 'opacity var(--dur-slow) var(--ease), transform var(--dur-slow) var(--ease)',
      pointerEvents: 'none',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    style: {
      width: '6px',
      height: '6px',
      borderRadius: 'var(--radius-full)',
      background: 'var(--accent)',
      flex: 'none'
    }
  }), icon, children);
}
Object.assign(__ds_scope, { Toast });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Toast.jsx", error: String((e && e.message) || e) }); }

// components/product/ActivityBars.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * ActivityBars — a compact request-activity sparkline (the stats panel's
 * "activity · 30m"). Zero-count buckets render as a dim baseline tick; active
 * buckets scale in green.
 */
function ActivityBars({
  data = [],
  height = 56,
  barWidth = 6,
  gap = 3,
  style,
  ...rest
}) {
  const max = Math.max(1, ...data);
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      display: 'flex',
      alignItems: 'flex-end',
      gap: `${gap}px`,
      height,
      ...style
    }
  }, rest), data.map((c, i) => {
    const h = c === 0 ? 3 : 6 + c / max * (height - 8);
    const idle = c === 0;
    return /*#__PURE__*/React.createElement("span", {
      key: i,
      title: `${c} req${c === 1 ? '' : 's'}`,
      style: {
        display: 'inline-block',
        width: barWidth,
        height: h,
        borderRadius: '1px',
        background: idle ? 'var(--ink-500)' : 'var(--ok)',
        opacity: idle ? 0.5 : 0.55 + h / height * 0.4,
        transition: 'height var(--dur) var(--ease)'
      }
    });
  }));
}
Object.assign(__ds_scope, { ActivityBars });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/ActivityBars.jsx", error: String((e && e.message) || e) }); }

// components/product/SearchPrompt.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * SearchPrompt — nxv's signature terminal-style search bar. Renders a prompt
 * sigil, the command word, an input, and a blinking caret; the whole shell
 * lifts with a focus ring on focus. Optionally shows an inline run button.
 */
function SearchPrompt({
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
  return /*#__PURE__*/React.createElement("form", _extends({
    onSubmit: e => {
      e.preventDefault();
      onSubmit && onSubmit(value);
    },
    style: {
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
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-md)',
      color: 'var(--accent)',
      userSelect: 'none'
    }
  }, host, "\xA0", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-subtle)'
    }
  }, ":~"), "$"), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-md)',
      color: 'var(--text-muted)',
      userSelect: 'none'
    }
  }, command), /*#__PURE__*/React.createElement("input", {
    type: "search",
    value: value,
    onChange: e => onChange && onChange(e.target.value),
    onFocus: () => setFocused(true),
    onBlur: () => setFocused(false),
    placeholder: placeholder,
    "aria-label": "Search nixpkgs by package and version",
    style: {
      flex: 1,
      minWidth: 0,
      background: 'transparent',
      border: 0,
      outline: 'none',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-md)',
      color: 'var(--text-heading)'
    }
  }), !button && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-lg)',
      color: 'var(--accent)',
      animation: 'nxv-caret 1.1s steps(1) infinite'
    }
  }, "\u258C"), button && /*#__PURE__*/React.createElement("button", {
    type: "submit",
    style: {
      height: '42px',
      padding: '0 20px',
      border: 0,
      borderRadius: 'var(--radius-md)',
      background: 'var(--accent-solid)',
      color: 'var(--accent-on)',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-base)',
      fontWeight: 'var(--weight-semibold)',
      cursor: 'pointer'
    }
  }, "search"), /*#__PURE__*/React.createElement("style", null, `@keyframes nxv-caret{0%,49%{opacity:1}50%,100%{opacity:0}}`));
}
Object.assign(__ds_scope, { SearchPrompt });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/SearchPrompt.jsx", error: String((e && e.message) || e) }); }

// components/product/Terminal.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Terminal — a window-chrome card (traffic-light dots + title) wrapping mono
 * content. Use for command demos, the how-it-works pipeline, and self-host
 * snippets. Pass pre-formatted children (spans with token colors) as content.
 */
function Terminal({
  title = 'zsh',
  children,
  style,
  ...rest
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-xl)',
      overflow: 'hidden',
      background: 'var(--surface-code)',
      boxShadow: 'var(--shadow-lg)',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      height: '44px',
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      padding: '0 18px',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 11,
      height: 11,
      borderRadius: '50%',
      background: '#ff5f56'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 11,
      height: 11,
      borderRadius: '50%',
      background: '#ffbd2e'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 11,
      height: 11,
      borderRadius: '50%',
      background: '#27c93f'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 12,
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)'
    }
  }, title)), /*#__PURE__*/React.createElement("pre", {
    style: {
      margin: 0,
      padding: '24px 26px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-base)',
      lineHeight: 'var(--leading-code)',
      color: 'var(--text)',
      whiteSpace: 'pre-wrap',
      overflowX: 'auto'
    }
  }, children));
}

/** Token color spans for use inside <Terminal> content. */
Terminal.Prompt = p => /*#__PURE__*/React.createElement("span", _extends({
  style: {
    color: 'var(--accent)'
  }
}, p));
Terminal.Comment = p => /*#__PURE__*/React.createElement("span", _extends({
  style: {
    color: 'var(--text-muted)'
  }
}, p));
Terminal.Hash = p => /*#__PURE__*/React.createElement("span", _extends({
  style: {
    color: 'var(--ok)'
  }
}, p));
Terminal.Emph = p => /*#__PURE__*/React.createElement("span", _extends({
  style: {
    color: 'var(--text-heading)'
  }
}, p));
Object.assign(__ds_scope, { Terminal });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/Terminal.jsx", error: String((e && e.message) || e) }); }

// components/product/VersionBadge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const TONES = {
  brand: {
    color: 'var(--nix-300)',
    border: 'oklch(0.55 0.17 262 / 0.45)',
    bg: 'var(--nix-wash)'
  },
  warn: {
    color: 'var(--warn)',
    border: 'oklch(0.78 0.14 80 / 0.45)',
    bg: 'var(--amber-wash)'
  },
  danger: {
    color: 'var(--danger)',
    border: 'oklch(0.66 0.19 25 / 0.55)',
    bg: 'var(--red-wash)'
  },
  plain: {
    color: 'var(--text-strong)',
    border: 'var(--border)',
    bg: 'var(--surface-raised)'
  }
};

/**
 * VersionBadge — the tabular-nums version tag shown on package rows/cards.
 * Tone signals status: brand (current), warn (pre-flakes), danger (insecure).
 */
function VersionBadge({
  version,
  tone = 'brand',
  style,
  ...rest
}) {
  const t = TONES[tone] || TONES.brand;
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      padding: '3px 9px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-xs)',
      fontFeatureSettings: "'zero' 1",
      color: t.color,
      border: `1px solid ${t.border}`,
      background: t.bg,
      borderRadius: 'var(--radius-sm)',
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), version);
}
Object.assign(__ds_scope, { VersionBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/VersionBadge.jsx", error: String((e && e.message) || e) }); }

// components/product/PackageCard.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const ICONS = {
  flake: /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2"
  }, /*#__PURE__*/React.createElement("rect", {
    x: "9",
    y: "9",
    width: "13",
    height: "13",
    rx: "2"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
  })),
  run: /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2"
  }, /*#__PURE__*/React.createElement("polygon", {
    points: "5 3 19 12 5 21 5 3"
  })),
  history: /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2"
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "10"
  }), /*#__PURE__*/React.createElement("polyline", {
    points: "12 6 12 12 16 14"
  })),
  shield: /*#__PURE__*/React.createElement("svg", {
    width: "10",
    height: "10",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.5"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 8v4M12 16h.01"
  }))
};
const dt = {
  fontSize: '9px',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  color: 'var(--text-subtle)'
};
const dd = {
  margin: 0,
  fontFamily: 'var(--font-mono)',
  fontSize: 'var(--text-2xs)',
  color: 'var(--text-muted)'
};

/**
 * PackageCard — the grid/card presentation of a search result. Same data as
 * PackageRow, laid out with a header (name/attr + version), description,
 * flag/platform chips, a first/last/rev meta strip, and three actions.
 */
function PackageCard({
  pkg,
  onCopyFlake,
  onCopyRun,
  onHistory,
  style,
  ...rest
}) {
  const tone = pkg.insecure ? 'danger' : pkg.legacy ? 'warn' : 'brand';
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("article", _extends({
    tabIndex: 0,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
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
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      gap: '10px',
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("h3", {
    style: {
      margin: 0,
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-sm)',
      fontWeight: 'var(--weight-semibold)',
      color: 'var(--text-heading)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, pkg.name), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '2px 0 0',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      color: 'var(--text-subtle)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--ink-400)',
      marginRight: '4px'
    }
  }, "\u203A"), pkg.attr)), /*#__PURE__*/React.createElement(__ds_scope.VersionBadge, {
    version: pkg.version,
    tone: tone,
    style: {
      alignSelf: 'flex-start',
      flexShrink: 0
    }
  })), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: 0,
      fontSize: 'var(--text-xs)',
      lineHeight: 'var(--leading-normal)',
      color: 'var(--text)',
      minHeight: '34px',
      display: '-webkit-box',
      WebkitLineClamp: 2,
      WebkitBoxOrient: 'vertical',
      overflow: 'hidden'
    }
  }, pkg.description || '—'), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexWrap: 'wrap',
      gap: '4px'
    }
  }, pkg.insecure && /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    tone: "danger",
    size: "sm",
    icon: ICONS.shield
  }, "insecure"), pkg.legacy && /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    tone: "warn",
    size: "sm"
  }, "pre-flakes"), (pkg.platforms || []).slice(0, 3).map(p => /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    key: p,
    size: "sm"
  }, p)), pkg.license && /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    size: "sm"
  }, pkg.license)), /*#__PURE__*/React.createElement("dl", {
    style: {
      display: 'flex',
      gap: '14px',
      margin: '2px 0 0',
      paddingTop: '10px',
      borderTop: '1px dashed var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("dt", {
    style: dt
  }, "first"), /*#__PURE__*/React.createElement("dd", {
    style: dd
  }, pkg.first || '—')), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("dt", {
    style: dt
  }, "last"), /*#__PURE__*/React.createElement("dd", {
    style: dd
  }, pkg.last || '—')), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("dt", {
    style: dt
  }, "rev"), /*#__PURE__*/React.createElement("dd", {
    style: dd
  }, "#", (pkg.hash || '').slice(0, 7)))), /*#__PURE__*/React.createElement("footer", {
    style: {
      display: 'flex',
      gap: '6px'
    },
    onClick: e => e.stopPropagation()
  }, /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    iconLeft: ICONS.flake,
    style: {
      flex: 1
    },
    onClick: () => onCopyFlake && onCopyFlake(pkg)
  }, "flake"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    iconLeft: ICONS.run,
    style: {
      flex: 1
    },
    onClick: () => onCopyRun && onCopyRun(pkg)
  }, "run"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    iconLeft: ICONS.history,
    style: {
      flex: 1
    },
    onClick: () => onHistory && onHistory(pkg)
  }, "history")));
}
Object.assign(__ds_scope, { PackageCard });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/PackageCard.jsx", error: String((e && e.message) || e) }); }

// components/product/PackageRow.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const ICONS = {
  copy: /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2"
  }, /*#__PURE__*/React.createElement("rect", {
    x: "9",
    y: "9",
    width: "13",
    height: "13",
    rx: "2"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
  })),
  history: /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2"
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "10"
  }), /*#__PURE__*/React.createElement("polyline", {
    points: "12 6 12 12 16 14"
  })),
  shield: /*#__PURE__*/React.createElement("svg", {
    width: "10",
    height: "10",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.5"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 8v4M12 16h.01"
  }))
};
const actionBtn = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: '6px',
  padding: '6px 10px',
  fontFamily: 'var(--font-mono)',
  fontSize: 'var(--text-xs)',
  color: 'var(--text-muted)',
  background: 'transparent',
  border: '1px solid var(--border-subtle)',
  borderRadius: 'var(--radius-sm)',
  cursor: 'pointer',
  transition: 'var(--transition)'
};

/**
 * PackageRow — a dense, scannable search-result row: package · attr, version,
 * description with platform/flag chips, first/last dates, and copy/history
 * actions. Built on Chip + VersionBadge. Meant for a headed list container.
 */
function PackageRow({
  pkg,
  onCopy,
  onHistory,
  style,
  ...rest
}) {
  const tone = pkg.insecure ? 'danger' : pkg.legacy ? 'warn' : 'brand';
  return /*#__PURE__*/React.createElement("div", _extends({
    style: {
      display: 'grid',
      gridTemplateColumns: 'minmax(180px,1.6fr) 100px minmax(200px,2fr) 120px 90px',
      gap: '16px',
      alignItems: 'center',
      padding: '14px 20px',
      borderBottom: '1px solid var(--border-subtle)',
      cursor: 'pointer',
      transition: 'background var(--dur) var(--ease)',
      ...style
    },
    onMouseEnter: e => e.currentTarget.style.background = 'var(--surface-raised)',
    onMouseLeave: e => e.currentTarget.style.background = 'transparent',
    onClick: () => onHistory && onHistory(pkg)
  }, rest), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: '8px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-sm)',
      fontWeight: 'var(--weight-medium)',
      color: 'var(--text-heading)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, pkg.name), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      color: 'var(--accent)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, pkg.attr)), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: '3px',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-2xs)',
      color: 'var(--text-subtle)',
      display: 'flex',
      gap: '8px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      maxWidth: '180px'
    }
  }, pkg.license || '—'), /*#__PURE__*/React.createElement("span", null, "\xB7"), /*#__PURE__*/React.createElement("span", null, "#", (pkg.hash || '').slice(0, 7)))), /*#__PURE__*/React.createElement(__ds_scope.VersionBadge, {
    version: pkg.version,
    tone: tone
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 'var(--text-sm)',
      color: 'var(--text)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, pkg.description || '—'), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: '6px',
      display: 'flex',
      flexWrap: 'wrap',
      gap: '4px'
    }
  }, pkg.insecure && /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    tone: "danger",
    size: "sm",
    icon: ICONS.shield
  }, "insecure"), pkg.legacy && /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    tone: "warn",
    size: "sm"
  }, "pre-flakes"), (pkg.platforms || []).slice(0, 3).map(p => /*#__PURE__*/React.createElement(__ds_scope.Chip, {
    key: p,
    size: "sm"
  }, p)))), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)',
      fontFeatureSettings: "'zero' 1"
    }
  }, pkg.first || '—'), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'flex-end',
      gap: '6px'
    },
    onClick: e => e.stopPropagation()
  }, /*#__PURE__*/React.createElement("button", {
    style: actionBtn,
    title: "copy flake ref",
    onClick: () => onCopy && onCopy(pkg)
  }, ICONS.copy), /*#__PURE__*/React.createElement("button", {
    style: actionBtn,
    title: "version history",
    onClick: () => onHistory && onHistory(pkg)
  }, ICONS.history)));
}
Object.assign(__ds_scope, { PackageRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/PackageRow.jsx", error: String((e && e.message) || e) }); }

// components/product/VersionTimeline.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * VersionTimeline — horizontal lifespan bars for a package's versions across a
 * shared time axis, with year gridlines and a dashed "flakes epoch" marker
 * (2020). Insecure versions render red. Drives the history drawer.
 */
function VersionTimeline({
  versions = [],
  height = 132,
  style,
  ...rest
}) {
  const times = [];
  versions.forEach(v => {
    const f = +new Date(v.first),
      l = +new Date(v.last);
    if (isFinite(f)) times.push(f);
    if (isFinite(l)) times.push(l);
  });
  const now = Date.now();
  const rawStart = times.length ? Math.min(...times) : now - 3.15e10;
  const rawEnd = times.length ? Math.max(...times) : now;
  const minSpan = 90 * 864e5;
  const span = Math.max(rawEnd - rawStart, minSpan);
  const pad = Math.max(14 * 864e5, span * 0.04);
  const A = rawStart - pad,
    B = rawEnd + pad,
    S = B - A;
  const x = t => (t - A) / S * 1000;
  const y0 = new Date(A).getUTCFullYear(),
    y1 = new Date(B).getUTCFullYear();
  const rows = versions.slice(0, 12);
  const rowH = 10,
    gap = 2;
  const h = Math.max(rows.length * (rowH + gap), 100);
  const flakeT = +new Date('2020-03-26T00:00:00Z');
  const years = [];
  for (let y = y0; y <= y1; y++) years.push(y);
  return /*#__PURE__*/React.createElement("div", _extends({
    style: style
  }, rest), /*#__PURE__*/React.createElement("svg", {
    viewBox: `0 0 1000 ${h}`,
    preserveAspectRatio: "none",
    style: {
      width: '100%',
      height,
      display: 'block'
    }
  }, years.map(y => {
    const gx = x(+new Date(Date.UTC(y, 0, 1)));
    if (gx < 0 || gx > 1000) return null;
    return /*#__PURE__*/React.createElement("line", {
      key: y,
      x1: gx,
      x2: gx,
      y1: 0,
      y2: h,
      stroke: "var(--ink-600)",
      strokeDasharray: "2 4",
      strokeWidth: "1"
    });
  }), flakeT >= A && flakeT <= B && /*#__PURE__*/React.createElement("line", {
    x1: x(flakeT),
    x2: x(flakeT),
    y1: 0,
    y2: h,
    stroke: "var(--amber)",
    strokeDasharray: "3 3",
    strokeWidth: "1",
    opacity: "0.5"
  }), rows.map((v, i) => {
    const x1 = Math.max(0, x(+new Date(v.first)));
    const x2 = Math.min(1000, x(+new Date(v.last)));
    const w = Math.max(2, x2 - x1);
    const yy = i * (rowH + gap);
    return /*#__PURE__*/React.createElement("g", {
      key: i
    }, /*#__PURE__*/React.createElement("rect", {
      x: x1,
      y: yy,
      width: w,
      height: rowH,
      rx: "1",
      fill: v.insecure ? 'var(--red)' : 'var(--nix-500)',
      opacity: v.insecure ? 0.75 : 0.85
    }), w > 30 && /*#__PURE__*/React.createElement("text", {
      x: Math.min(980, x2 + 4),
      y: yy + rowH - 1.5,
      fontFamily: "var(--font-mono)",
      fontSize: "8.5",
      fill: "var(--text-muted)"
    }, v.version));
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: '8px',
      display: 'flex',
      justifyContent: 'space-between',
      fontFamily: 'var(--font-mono)',
      fontSize: '10px',
      color: 'var(--text-subtle)',
      fontFeatureSettings: "'zero' 1"
    }
  }, years.filter((_, i) => i % Math.max(1, Math.ceil(years.length / 10)) === 0).map(y => /*#__PURE__*/React.createElement("span", {
    key: y
  }, "'", String(y).slice(2)))));
}
Object.assign(__ds_scope, { VersionTimeline });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/product/VersionTimeline.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Callout = __ds_scope.Callout;

__ds_ns.Chip = __ds_scope.Chip;

__ds_ns.CommandPalette = __ds_scope.CommandPalette;

__ds_ns.Kbd = __ds_scope.Kbd;

__ds_ns.Metric = __ds_scope.Metric;

__ds_ns.Pagination = __ds_scope.Pagination;

__ds_ns.Panel = __ds_scope.Panel;

__ds_ns.SegmentedToggle = __ds_scope.SegmentedToggle;

__ds_ns.StatusPill = __ds_scope.StatusPill;

__ds_ns.Toast = __ds_scope.Toast;

__ds_ns.ActivityBars = __ds_scope.ActivityBars;

__ds_ns.PackageCard = __ds_scope.PackageCard;

__ds_ns.PackageRow = __ds_scope.PackageRow;

__ds_ns.SearchPrompt = __ds_scope.SearchPrompt;

__ds_ns.Terminal = __ds_scope.Terminal;

__ds_ns.VersionBadge = __ds_scope.VersionBadge;

__ds_ns.VersionTimeline = __ds_scope.VersionTimeline;

})();
