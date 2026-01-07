import nextra from 'nextra'

const withNextra = nextra({})

/** @type {import('next').NextConfig} */
const nextConfig = {
    output: 'export',
}

export default withNextra(nextConfig)
