import React from "react"
import type { Metadata } from 'next'
import { Analytics } from '@vercel/analytics/next'
import { Toaster } from 'sonner'
import { appConfig } from '@/lib/app-config'
import { ErrorBoundary } from '@/components/error-boundary'
import './globals.css'

export const metadata: Metadata = {
  title: appConfig.name,
  description: appConfig.description,
  icons: {
    icon: [
      { url: appConfig.favicon32, sizes: '32x32', type: 'image/png' },
      { url: appConfig.favicon16, sizes: '16x16', type: 'image/png' },
    ],
    apple: appConfig.appleTouchIcon,
  },
  manifest: appConfig.manifest,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body className={`font-sans antialiased`}>
        <ErrorBoundary>{children}</ErrorBoundary>
        <Toaster theme="dark" richColors />
        <Analytics />
      </body>
    </html>
  )
}
