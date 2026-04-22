import { defineConfig } from 'vitepress'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  title: 'nxv',
  description: 'Find any version of any Nix package',
  base: '/nxv/',

  vite: {
    plugins: [tailwindcss()],
    server: {
      // Fall through to the next free port if 5173 is busy.
      strictPort: false,
    },
  },

  head: [
    ['link', { rel: 'icon', href: '/nxv/nxv-logo-dark.svg' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.googleapis.com' }],
    [
      'link',
      {
        rel: 'preconnect',
        href: 'https://fonts.gstatic.com',
        crossorigin: '',
      },
    ],
    [
      'link',
      {
        rel: 'stylesheet',
        href: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Inter:wght@400;500;600;700;800&display=swap',
      },
    ],
  ],

  themeConfig: {
    logo: {
      light: '/nxv-logo-light.svg',
      dark: '/nxv-logo-dark.svg',
      alt: 'nxv',
    },
    siteTitle: false,

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
