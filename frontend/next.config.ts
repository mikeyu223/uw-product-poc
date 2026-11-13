import type { NextConfig } from "next";

/** Server-only backend origin for same-origin proxy (keeps UW key off the client). */
const backendUrl = (process.env.BACKEND_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "").replace(
  /\/$/,
  "",
);

const nextConfig: NextConfig = {
  async rewrites() {
    if (!backendUrl) return [];
    return [
      { source: "/health", destination: `${backendUrl}/health` },
      { source: "/api/v1/:path*", destination: `${backendUrl}/api/v1/:path*` },
    ];
  },
};

export default nextConfig;
