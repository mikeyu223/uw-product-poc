import type { Methodology, PortfolioSummary, Position, ProbeResponse } from "./types";

/**
 * Browser calls same-origin paths in production (Next rewrites → Rust backend).
 * Locally, hit the Axum server directly unless NEXT_PUBLIC_API_URL is set.
 */
const API_BASE =
  process.env.NEXT_PUBLIC_API_URL ??
  (process.env.NODE_ENV === "production" ? "" : "http://127.0.0.1:8080");


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

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`${path} failed (${res.status})`);
  }
  return res.json() as Promise<T>;
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
