/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  typescript: {
    ignoreBuildErrors: false,
  },
  images: {
    unoptimized: true,
  },
  async rewrites() {
    const rustBackend = process.env.RUST_BACKEND_URL;
    if (!rustBackend) return { beforeFiles: [] };
    try {
      new URL(rustBackend);
    } catch {
      throw new Error(`Invalid RUST_BACKEND_URL: "${rustBackend}" — must be a valid URL (e.g., http://rust-backend:8080)`);
    }
    return {
      beforeFiles: [
        {
          source: "/api/:path*",
          destination: `${rustBackend}/api/:path*`,
        },
      ],
    };
  },
};

export default nextConfig;
