import type {
  CvdProfile,
  EarningsIvReport,
  FuturesContract,
  JournalAnalytics,
  JournalFilters,
  OiMatrixReport,
  SweepClusterReport,
  TradeChart,
  TradeEntry,
} from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "";

function joinUrl(base: string, path: string): string {
  if (!base) return path;
  return `${base.replace(/\/$/, "")}${path.startsWith("/") ? path : `/${path}`}`;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(joinUrl(API_BASE, path), { cache: "no-store" });
  if (!res.ok) {
    let detail = `${path} failed (${res.status})`;
    try {
      const err = (await res.json()) as { error?: string };
      if (err.error) detail = err.error;
    } catch {
      /* ignore */
    }
    throw new Error(detail);
  }
  return res.json() as Promise<T>;
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(joinUrl(API_BASE, path), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    cache: "no-store",
  });
  if (!res.ok) {
    let detail = `${path} failed (${res.status})`;
    try {
      const err = (await res.json()) as { error?: string };
      if (err.error) detail = err.error;
    } catch {
      /* ignore */
    }
    throw new Error(detail);
  }
  return res.json() as Promise<T>;
}

export async function fetchHealth(): Promise<{ status: string; features?: string[] }> {
  return getJson("/health");
}

export async function fetchFuturesContracts(): Promise<FuturesContract[]> {
  const data = await getJson<{ contracts: FuturesContract[] }>("/api/v1/futures/contracts");
  return data.contracts;
}

export async function fetchCvdProfile(contract: string, limit = 500): Promise<CvdProfile> {
  const q = new URLSearchParams({ limit: String(limit) });
  const data = await getJson<{ profile: CvdProfile }>(
    `/api/v1/futures/${encodeURIComponent(contract)}/cvd?${q.toString()}`,
  );
  return data.profile;
}

export async function fetchOiMatrix(): Promise<OiMatrixReport> {
  const data = await getJson<{ report: OiMatrixReport }>("/api/v1/futures/oi-matrix");
  return data.report;
}

export async function fetchEarningsIv(tickers?: string[]): Promise<EarningsIvReport> {
  const q = tickers?.length ? `?tickers=${encodeURIComponent(tickers.join(","))}` : "";
  const data = await getJson<{ report: EarningsIvReport }>(`/api/v1/options/earnings-iv${q}`);
  return data.report;
}

export async function fetchSweepClusters(limit = 200): Promise<SweepClusterReport> {
  const q = new URLSearchParams({ limit: String(limit) });
  const data = await getJson<{ report: SweepClusterReport }>(
    `/api/v1/options/sweep-clusters?${q.toString()}`,
  );
  return data.report;
}

export async function fetchJournalAnalytics(
  filters: JournalFilters = {},
): Promise<JournalAnalytics> {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(filters)) {
    if (v) q.set(k, v);
  }
  const qs = q.toString();
  const data = await getJson<{ analytics: JournalAnalytics }>(
    `/api/v1/journal/analytics${qs ? `?${qs}` : ""}`,
  );
  return data.analytics;
}

export async function fetchTradeChart(id: string): Promise<TradeChart> {
  const data = await getJson<{ chart: TradeChart }>(
    `/api/v1/journal/${encodeURIComponent(id)}/chart`,
  );
  return data.chart;
}

export async function fetchJournalTrades(): Promise<TradeEntry[]> {
  const data = await getJson<{ trades: TradeEntry[] }>("/api/v1/journal");
  return data.trades;
}

export async function importJournalCsv(csv: string): Promise<{
  imported: number;
  analytics: JournalAnalytics;
  trades: TradeEntry[];
}> {
  return postJson("/api/v1/journal/import", { csv });
}

export async function patchJournalTrade(
  id: string,
  body: {
    notes?: string;
    emotion?: string;
    setup?: string;
    tags?: string[];
    lesson?: string;
    followed_rules?: boolean;
  },
): Promise<TradeEntry> {
  const data = await postJson<{ trade: TradeEntry }>(
    `/api/v1/journal/${encodeURIComponent(id)}`,
    body,
  );
  return data.trade;
}

export async function addJournalTrade(body: {
  symbol: string;
  side: string;
  qty: number;
  entry_price: number;
  exit_price: number;
  account?: string;
  asset_class?: string;
  commissions?: number;
  setup?: string;
  notes?: string;
  emotion?: string;
  tags?: string[];
  dte_at_entry?: number;
  iv_rank_at_entry?: number;
  contract_strategy?: string;
}): Promise<TradeEntry> {
  const data = await postJson<{ trade: TradeEntry }>("/api/v1/journal", body);
  return data.trade;
}
