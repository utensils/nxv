import { defineConfig } from 'vitepress'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  title: 'nxv',
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
    // Single dark Shiki palette — the site never renders light.
    theme: 'github-dark',
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
