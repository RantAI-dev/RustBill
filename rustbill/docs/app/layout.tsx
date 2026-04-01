import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import type { Metadata } from 'next'
import type { ReactNode } from 'react'
import 'nextra-theme-docs/style.css'
import './custom.css'

export const metadata: Metadata = {
  title: {
    default: 'RustBill Docs',
    template: '%s | RustBill Docs',
  },
  description:
    'RustBill — Open-Source Billing, Product & License Management API built with Rust.',
  icons: {
    icon: [
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
    ],
    shortcut: '/favicon.ico',
    apple: '/apple-touch-icon.png',
  },
  openGraph: {
    title: 'RustBill Documentation',
    description:
      'Open-source billing, subscription, and license management API built with Rust.',
    siteName: 'RustBill Docs',
    type: 'website',
  },
}

const navbar = (
  <Navbar
    logo={
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src="/rustbill-logo.png"
          alt="RustBill"
          width={28}
          height={28}
          style={{ borderRadius: 6 }}
        />
        <span
          style={{
            fontWeight: 800,
            fontSize: '1.1rem',
            letterSpacing: '-0.02em',
          }}
        >
          RustBill
        </span>
        <span
          style={{
            fontSize: '0.7rem',
            fontWeight: 500,
            opacity: 0.5,
            textTransform: 'uppercase',
            letterSpacing: '0.08em',
            marginLeft: -4,
          }}
        >
          docs
        </span>
      </div>
    }
    projectLink="https://github.com/RantAI-dev/RustBill"
  />
)

const footer = (
  <Footer>
    <div
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        width: '100%',
        flexWrap: 'wrap',
        gap: '0.5rem',
      }}
    >
      <span>MIT {new Date().getFullYear()} &copy; RantAI</span>
      <div style={{ display: 'flex', gap: '1.5rem', fontSize: '0.875rem' }}>
        <a
          href="https://github.com/RantAI-dev/RustBill"
          target="_blank"
          rel="noopener noreferrer"
          style={{ opacity: 0.6 }}
        >
          GitHub
        </a>
        <a
          href="https://github.com/RantAI-dev/RustBill/issues"
          target="_blank"
          rel="noopener noreferrer"
          style={{ opacity: 0.6 }}
        >
          Issues
        </a>
        <a
          href="https://github.com/RantAI-dev/RustBill/releases"
          target="_blank"
          rel="noopener noreferrer"
          style={{ opacity: 0.6 }}
        >
          Releases
        </a>
      </div>
    </div>
  </Footer>
)

export default async function RootLayout({
  children,
}: {
  children: ReactNode
}) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/RantAI-dev/RustBill/tree/main/docs"
          footer={footer}
          sidebar={{ defaultMenuCollapseLevel: 1 }}
          editLink="Edit this page on GitHub"
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
