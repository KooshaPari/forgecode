import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'forgecode',
  description: 'Forgecode',
  outDir: '../docs-dist',
  themeConfig: {
    nav: [{ text: 'Home', link: '/' }],
    sidebar: [{ text: 'Overview', link: '/' }]
  }
})
