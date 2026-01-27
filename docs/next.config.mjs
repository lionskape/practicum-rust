import nextra from 'nextra'

const withNextra = nextra({})

/** @type {import('next').NextConfig} */
const nextConfig = {
    output: 'export',
    basePath: '/practicum-rust',
    assetPrefix: '/practicum-rust/',
}

export default withNextra(nextConfig)
