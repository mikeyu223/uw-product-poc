import type { Methodology, PortfolioSummary, Position, ProbeResponse } from "./types";

/**
 * Browser always uses same-origin paths (`/api/v1/...`, `/health`).
 * - Vercel: root `vercel.json` rewrites those to the internal `backend` service.
 * - Local `vercel dev`: same rewrites + injected `BACKEND_URL` binding.
 * - Local `next dev` + `cargo run`: set NEXT_PUBLIC_API_URL=http://127.0.0.1:8080
 *   OR set BACKEND_URL and rely on next.config rewrites with empty API_BASE.
 *
 * Server-side code (Route Handlers only) may call `process.env.BACKEND_URL`
 * from the frontend→backend service binding. Never expose the UW API key
 * to the client.
 */
const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "";

export interface CacheMeta {
  hit: boolean;
  age_secs: number;
  ttl_secs: number;
  fetched_at: string;
}

export interface BookResponse {
  positions: Position[];
  summary: PortfolioSummary;
  mode: string;
  cache: CacheMeta;
  note?: string;
}

function joinUrl(base: string, path: string): string {
  if (!base) return path;
  return `${base.replace(/\/$/, "")}${path.startsWith("/") ? path : `/${path}`}`;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(joinUrl(API_BASE, path), {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`${path} failed (${res.status})`);
  }
  return res.json() as Promise<T>;
}

/** Server-only helper: call the bound backend service over BACKEND_URL. */
export function backendUrl(path = ""): string {
  const base = process.env.BACKEND_URL;
  if (!base) {
    throw new Error("BACKEND_URL is not set (service binding missing)");
  }
  return new URL(path.replace(/^\//, ""), base.endsWith("/") ? base : `${base}/`).toString();
}

/** Single cached book fetch — preferred over separate portfolio+positions. */
export async function fetchBook(forceRefresh = false): Promise<BookResponse> {
  const q = forceRefresh ? "?refresh=1" : "";
  return getJson<BookResponse>(`/api/v1/book${q}`);
}

export async function fetchPortfolio(): Promise<PortfolioSummary> {
  const data = await getJson<{ summary: PortfolioSummary }>("/api/v1/portfolio");
  return data.summary;
}

export async function fetchPositions(): Promise<Position[]> {
  const data = await getJson<{ positions: Position[] }>("/api/v1/positions");
  return data.positions;
}

export async function fetchProbe(ticker: string): Promise<ProbeResponse> {
  return getJson<ProbeResponse>(`/api/v1/uw/probe/${encodeURIComponent(ticker)}`);
}

export async function fetchHealth(): Promise<{ status: string; cache_ttl_secs?: number }> {
  return getJson("/health");
}

export async function fetchMethodology(): Promise<Methodology> {
  return getJson("/api/v1/methodology");
}
