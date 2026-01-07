import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import type { ReactNode } from 'react'
import './globals.css'

export const metadata = {
  title: 'Practicum Rust',
  description: 'Документация практикума по Rust'
}

const navbar = (
  <Navbar
    logo={<strong>Practicum Rust</strong>}
    projectLink="https://github.com/your-org/practicum-rust"
  />
)

const footer = <Footer>MIT {new Date().getFullYear()} Practicum Rust</Footer>

export default async function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="ru" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          footer={footer}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/your-org/practicum-rust/tree/main/docs"
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
