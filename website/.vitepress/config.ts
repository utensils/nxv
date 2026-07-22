import { defineConfig } from 'vitepress'
import tailwindcss from '@tailwindcss/vite'

/**
 * Blueprint-mapped Shiki theme — mirrors the DS Terminal component's
 * restraint: accent for keywords, green for strings/hashes, muted fog
 * for comments, fog-1 for plain text, nix-300 for functions/constants.
 *
 * sRGB conversions of the Blueprint oklch tokens (code bg is
 * --ink-900 oklch(0.12 0.018 264) = #03060c):
 *   fog-1   #f0f2f6  (plain text, 18.1:1)
 *   fog-2   #b4bed2  (operators/punctuation, 10.9:1)
 *   fog-3   #8692ab  (comments, 6.5:1 — AA requires 4.5:1)
 *   green   #59d38c  (strings/hashes, 10.8:1)
 *   nix-300 #97c9fd  (functions/constants/types, 11.7:1)
 *   nix-400 #74a6ef  (keywords, 8.2:1)
 *   red     #f05653  (diff deletions, 6.0:1)
 */
const blueprintCodeTheme = {
  name: 'nxv-blueprint',
  type: 'dark' as const,
  colors: {
    'editor.background': '#03060c',
    'editor.foreground': '#f0f2f6',
  },
  settings: [
    {
      settings: { background: '#03060c', foreground: '#f0f2f6' },
    },
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: '#8692ab', fontStyle: 'italic' },
    },
    {
      scope: [
        'string',
        'punctuation.definition.string',
        'constant.other.symbol',
        'markup.inserted',
      ],
      settings: { foreground: '#59d38c' },
    },
    {
      scope: [
        'constant.numeric',
        'constant.language',
        'constant.character',
        'constant.other',
        'support.constant',
        'variable.other.constant',
      ],
      settings: { foreground: '#97c9fd' },
    },
    {
      scope: [
        'keyword',
        'storage',
        'storage.type',
        'storage.modifier',
        'entity.name.tag',
      ],
      settings: { foreground: '#74a6ef' },
    },
    {
      scope: ['keyword.operator', 'punctuation'],
      settings: { foreground: '#b4bed2' },
    },
    {
      scope: ['entity.name.function', 'support.function'],
      settings: { foreground: '#97c9fd' },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'entity.other.inherited-class',
        'support.type',
        'support.class',
      ],
      settings: { foreground: '#97c9fd' },
    },
    {
      scope: [
        'entity.other.attribute-name',
        'support.type.property-name',
        'meta.property-name',
      ],
      settings: { foreground: '#97c9fd' },
    },
    {
      scope: ['variable', 'variable.parameter'],
      settings: { foreground: '#f0f2f6' },
    },
    {
      scope: ['markup.heading', 'entity.name.section'],
      settings: { foreground: '#74a6ef', fontStyle: 'bold' },
    },
    {
      scope: ['markup.deleted'],
      settings: { foreground: '#f05653' },
    },
    {
      scope: ['markup.bold'],
      settings: { fontStyle: 'bold' },
    },
    {
      scope: ['markup.italic'],
      settings: { fontStyle: 'italic' },
    },
  ],
}

export default defineConfig({
  title: 'nxv',
  // Section pages render as ':title | nxv docs'; the home page carries
  // the canonical slogan via its own frontmatter titleTemplate.
  titleTemplate: ':title | nxv docs',
  description: 'Find any version of any Nix package, instantly.',
  base: '/nxv/',

  // Blueprint is dark-only by design — pin dark and drop the toggle.
  appearance: 'force-dark',

  vite: {
    plugins: [tailwindcss()],
    server: {
      // Fall through to the next free port if 5173 is busy.
      strictPort: false,
    },
  },

  markdown: {
    // Single dark Blueprint palette — the site never renders light.
    theme: blueprintCodeTheme,
  },

  head: [
    ['link', { rel: 'icon', href: '/nxv/nix-snowflake.svg' }],
    [
      'link',
      {
        rel: 'preload',
        href: '/nxv/fonts/inter-var.woff2',
        as: 'font',
        type: 'font/woff2',
        crossorigin: '',
      },
    ],
    [
      'link',
      {
        rel: 'preload',
        href: '/nxv/fonts/jetbrains-mono-var.woff2',
        as: 'font',
        type: 'font/woff2',
        crossorigin: '',
      },
    ],
  ],

  themeConfig: {
    logo: { src: '/nix-snowflake.svg', alt: 'nxv' },
    siteTitle: 'nxv',

    nav: [
      { text: 'Guide', link: '/guide/' },
      { text: 'API', link: '/api/' },
      { text: 'Advanced', link: '/advanced/indexer' },
      { text: 'GitHub', link: 'https://github.com/utensils/nxv' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'Getting Started', link: '/guide/' },
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Configuration', link: '/guide/configuration' },
          ],
        },
        {
          text: 'Usage',
          items: [{ text: 'CLI Reference', link: '/guide/cli-reference' }],
        },
        {
          text: 'Integrations',
          items: [{ text: 'Agent Skill', link: '/guide/skill' }],
        },
      ],
      '/advanced/': [
        {
          text: 'Advanced',
          items: [
            { text: 'Building Indexes', link: '/advanced/indexer' },
            { text: 'Indexer CLI Reference', link: '/advanced/indexer-cli' },
            { text: 'Troubleshooting', link: '/advanced/troubleshooting' },
          ],
        },
      ],
      '/api/': [
        {
          text: 'API Reference',
          items: [{ text: 'HTTP API', link: '/api/' }],
        },
      ],
    },

    socialLinks: [{ icon: 'github', link: 'https://github.com/utensils/nxv' }],

    search: {
      provider: 'local',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright James Brink',
    },

    editLink: {
      pattern: 'https://github.com/utensils/nxv/edit/main/website/:path',
      text: 'Edit this page on GitHub',
    },
  },
})
