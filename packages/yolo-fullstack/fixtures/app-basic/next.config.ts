import type { NextConfig } from "../../src/config/next-config";

const config: NextConfig = {
  poweredByHeader: true,
  env: { APP_NAME: "fixture" },
  images: {
    remotePatterns: [{ protocol: "http", hostname: "127.0.0.1" }],
    deviceSizes: [640, 1200],
    imageSizes: [64],
  },
  turbopack: {
    resolveAlias: { "@ui/*": "./ui/*", "#alias-target": "./ui/alias.ts" },
    rules: { "*.txt": { loaders: ["raw-loader"], as: "*.js" } },
  },
  async rewrites() {
    return { beforeFiles: [{ source: "/rewritten", destination: "/about" }], afterFiles: [{ source: "/legacy/:slug", destination: "/blog/:slug" }], fallback: [] };
  },
  async redirects() {
    return [{ source: "/old", destination: "/about", permanent: true }];
  },
  async headers() {
    return [{ source: "/(.*)", headers: [{ key: "x-frame-options", value: "DENY" }] }];
  },
  webpack: () => ({}),
};

export default config;
