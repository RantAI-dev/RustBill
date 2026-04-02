import nextra from 'nextra'

const withNextra = nextra({
  contentDirBasePath: '/',
})

const isDev = process.env.NODE_ENV === 'development'

export default withNextra({
  ...(!isDev && { output: 'export' }),
  images: {
    unoptimized: true,
  },
  outputFileTracingRoot: import.meta.dirname,
  basePath: process.env.PAGES_BASE_PATH || '',
})
