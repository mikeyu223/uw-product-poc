import type { NextConfig } from "next";

/**
 * On Vercel Services, public routing lives in root `vercel.json` rewrites
 * (browser → `/api/*` and `/health` → `backend` service).
 *
 * `BACKEND_URL` is injected via a service binding for server-side use only
 * (Route Handlers / Server Components). Do not set it yourself on Vercel.
 *
 * These Next rewrites are a local/dev fallback when you point FRONTEND at a
 * separately running Axum process via BACKEND_URL or NEXT_PUBLIC_API_URL.
 */
const backendUrl = (process.env.BACKEND_URL ?? "").replace(/\/$/, "");

const nextConfig: NextConfig = {
  async rewrites() {
    // Prefer same-origin + vercel.json when no explicit backend URL is set.
    if (!backendUrl) return [];
    return [
      { source: "/health", destination: `${backendUrl}/health` },
      { source: "/api/v1/:path*", destination: `${backendUrl}/api/v1/:path*` },
    ];
  },
};

export default nextConfig;
