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
  // Uncomment and set basePath when deploying to GitHub Pages under a subpath:
  // basePath: '/RustBill',
})
