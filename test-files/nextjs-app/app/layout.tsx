import './globals.css'
import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'Next.js App Router Test',
  description: 'Test app for Danny framework detection',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
