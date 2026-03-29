export function createSiteMeta({ base = '/' } = {}) {
  return {
    base,
    title: 'forgecode',
    description: 'Documentation',
    themeConfig: {
      nav: [
        { text: 'Home', link: base || '/' },
      ],
    },
  }
}
