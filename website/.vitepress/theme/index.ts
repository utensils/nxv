import { h } from 'vue'
import DefaultTheme from 'vitepress/theme-without-fonts'
import './style.css'

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      // Canonical Blueprint eyebrow above the home hero.
      'home-hero-info-before': () =>
        h('p', { class: 'nxv-eyebrow' }, '// nix version index'),
    }),
}
