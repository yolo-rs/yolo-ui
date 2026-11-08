import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  output: "standalone",
  basePath: "",
  assetPrefix: process.env.CDN_URL ?? "",
  trailingSlash: false,
  compress: true,
  poweredByHeader: false,
  reactCompiler: true,
  cacheComponents: true,
  typedRoutes: true,
  deploymentId: process.env.DEPLOYMENT_ID,
  transpilePackages: ["@acme/ui"],
  serverExternalPackages: ["sharp", "pino"],
  pageExtensions: ["tsx", "ts", "mdx"],
  env: { APP_NAME: "Yolo Fixture", API_BASE: "https://api.example.com" },
  turbopack: {
    root: process.cwd(),
    rules: {
      "*.svg": { loaders: ["@svgr/webpack"], as: "*.js" },
      "*.md": { loaders: ["raw-loader"], as: "*.js" },
      "*.graphql": { loaders: ["graphql-tag/loader"], as: "*.js" },
      "*.yaml": { loaders: ["yaml-loader"], as: "*.js" },
    },
    resolveAlias: {
      underscore: "lodash",
      mocha: { browser: "mocha/browser-entry.js" },
      fs: { browser: "./empty.ts" },
      "@/*": "./*",
    },
    resolveExtensions: [".mdx", ".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"],
  },
  images: {
    formats: ["image/avif", "image/webp"],
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    imageSizes: [32, 48, 64, 96, 128, 256, 384],
    qualities: [50, 75, 90],
    minimumCacheTTL: 86400,
    dangerouslyAllowSVG: false,
    remotePatterns: [
      { protocol: "https", hostname: "images.unsplash.com", pathname: "/**" },
      { protocol: "https", hostname: "**.cloudfront.net" },
      { protocol: "https", hostname: "cdn.example.com", port: "", pathname: "/media/**", search: "" },
    ],
    localPatterns: [{ pathname: "/assets/images/**", search: "" }],
  },
  async redirects() {
    return [
      { source: "/old-blog/:slug", destination: "/blog/:slug", permanent: true },
      { source: "/docs", destination: "/docs/getting-started", permanent: false },
      {
        source: "/legacy/:path*",
        destination: "https://legacy.example.com/:path*",
        permanent: false,
        has: [{ type: "header", key: "x-legacy", value: "1" }],
      },
    ];
  },
  async rewrites() {
    return {
      beforeFiles: [{ source: "/api/proxy/:path*", destination: "https://api.example.com/:path*" }],
      afterFiles: [{ source: "/u/:username", destination: "/users/:username" }],
      fallback: [{ source: "/:path*", destination: "https://fallback.example.com/:path*" }],
    };
  },
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: [
          { key: "X-Frame-Options", value: "DENY" },
          { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
        ],
      },
      {
        source: "/_next/static/:path*",
        headers: [{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }],
      },
    ];
  },
  compiler: {
    removeConsole: { exclude: ["error", "warn"] },
    define: { __BUILD_FLAG__: "true" },
  },
  cacheLife: { blog: { stale: 3600, revalidate: 900, expire: 86400 } },
  experimental: {
    serverActions: { bodySizeLimit: "2mb", allowedOrigins: ["app.example.com"] },
    staleTimes: { dynamic: 30, static: 180 },
    optimizePackageImports: ["lucide-react"],
    authInterrupts: true,
    viewTransition: true,
  },
  // Removed in Next.js 16 (next lint is gone); kept so the migrator reports it.
  eslint: { ignoreDuringBuilds: true },
  logging: { fetches: { fullUrl: true } },
};

export default nextConfig;
