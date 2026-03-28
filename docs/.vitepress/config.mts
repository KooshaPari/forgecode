import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid({
  title: 'Forgecode',
  description: 'AI-enabled pair programmer using gitoxide and modern Git tooling.',
  appearance: 'dark',
  lastUpdated: true,
  themeConfig: {
    nav: [{ text: 'Home', link: '/' }],
    sidebar: [],
    search: { provider: 'local' },
  },
  mermaid: { theme: 'dark' },
})
