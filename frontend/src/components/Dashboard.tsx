"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { fetchBook, fetchHealth, fetchMethodology, fetchProbe, type CacheMeta } from "@/lib/api";
import type { FamilyScore, Methodology, PortfolioSummary, Position, ProbeResponse } from "@/lib/types";
import {
  bucketBg,
  bucketColor,
  bucketLabel,
  formatCompactNumber,
  scoreBarHex,
} from "@/lib/risk";
import { Sidebar } from "./Sidebar";

const FALLBACK_FAMILIES: Array<{ family: string; label: string; max_points: number }> = [
  { family: "gamma_regime", label: "Dealer GEX regime", max_points: 26 },
  { family: "flip_proximity", label: "Zero-gamma flip proximity", max_points: 20 },
  { family: "wall_conflict", label: "Call/put wall conflict", max_points: 16 },
  { family: "adverse_flow", label: "Adverse options flow", max_points: 20 },
  { family: "structure", label: "Structure (DTE / moneyness)", max_points: 10 },
  { family: "catalyst", label: "Earnings / catalyst", max_points: 8 },
];

type DetailTab = "drivers" | "breakdown" | "probe";
type MainTab = "positions" | "mix" | "probe";

export function Dashboard() {
  const [backendOk, setBackendOk] = useState<boolean | null>(null);
  const [summary, setSummary] = useState<PortfolioSummary | null>(null);
  const [positions, setPositions] = useState<Position[]>([]);
  const [methodology, setMethodology] = useState<Methodology | null>(null);
  const [probe, setProbe] = useState<ProbeResponse | null>(null);
  const [probeTicker, setProbeTicker] = useState("SPY");
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [probing, setProbing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>("drivers");
  const [mainTab, setMainTab] = useState<MainTab>("positions");
  const [cache, setCache] = useState<CacheMeta | null>(null);
  const [search, setSearch] = useState("");

  const selected = useMemo(
    () => positions.find((p) => p.id === selectedId) ?? positions[0] ?? null,
    [positions, selectedId],
  );

  const filtered = useMemo(() => {
    const q = search.trim().toUpperCase();
    if (!q) return positions;
    return positions.filter(
      (p) =>
        p.ticker.includes(q) ||
        p.option_type.toUpperCase().includes(q) ||
        String(p.strike).includes(q),
    );
  }, [positions, search]);

  const load = useCallback(async (forceRefresh = false) => {
    setError(null);
    if (forceRefresh) setRefreshing(true);
    else setLoading(true);
    try {
      const health = await fetchHealth();
      setBackendOk(health.status === "ok");
      const [book, m] = await Promise.all([
        fetchBook(forceRefresh),
        fetchMethodology().catch(() => null),
      ]);
      setSummary(book.summary);
      setPositions(book.positions);
      setCache(book.cache);
      if (m) setMethodology(m);
      if (book.positions[0]) setSelectedId((prev) => prev ?? book.positions[0].id);
    } catch (e) {
      setBackendOk(false);
      setError(e instanceof Error ? e.message : "Failed to load dashboard data");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  async function runProbe(ticker?: string) {
    const t = (ticker ?? probeTicker).trim() || "SPY";
    setProbing(true);
    try {
      const data = await fetchProbe(t);
      setProbe(data);
    } catch (e) {
      setProbe(null);
      setError(e instanceof Error ? e.message : "UW probe failed");
    } finally {
      setProbing(false);
    }
  }

  useEffect(() => {
    if (backendOk) void runProbe("SPY");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backendOk]);

  useEffect(() => {
    if (selected?.ticker) setProbeTicker(selected.ticker);
  }, [selected?.ticker]);

  const live = summary?.mode === "live_uw";

  const peak = useMemo(() => {
    if (positions.length === 0) return null;
    return positions.reduce((worst, p) => (p.score > worst.score ? p : worst), positions[0]);
  }, [positions]);

  const bookHealth = useMemo(() => {
    const critical = positions.filter((p) => p.bucket === "high").length;
    const elevated = positions.filter((p) => p.bucket === "elevated").length;
    return {
      count: positions.length,
      critical,
      elevated,
    };
  }, [positions]);

  const topWhy = useMemo(() => {
    const fromPeak = peak?.drivers?.[0];
    if (fromPeak) {
      return {
        ticker: peak.ticker,
        label: fromPeak.label,
      };
    }
    const fromSummary = summary?.top_drivers?.[0];
    if (fromSummary) {
      return {
        ticker: fromSummary.ticker ?? "",
        label: fromSummary.label,
      };
    }
    return null;
  }, [peak, summary]);

  return (
    <div className="flex h-screen overflow-hidden bg-[var(--bg-soft)]">
      <Sidebar backendOk={backendOk} />

      <div className="flex min-w-0 flex-1 flex-col">
        {/* Top nav — Public-style */}
        <header className="flex shrink-0 items-center gap-6 border-b border-[var(--border)] bg-white px-5 py-3">
          <div className="text-[20px] font-extrabold tracking-tight text-[var(--ink)]">
            risk<span className="text-[var(--accent)]">watcher</span>
          </div>

          <div className="relative hidden min-w-[220px] max-w-md flex-1 md:block">
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search assets"
              className="w-full rounded-full border border-[var(--border)] bg-[var(--bg-soft)] px-4 py-2 text-sm text-[var(--text)] outline-none placeholder:text-[var(--text-dim)] focus:border-[var(--accent)]"
            />
          </div>

          <nav className="hidden items-center gap-5 text-[14px] font-medium text-[var(--text-muted)] lg:flex">
            <span className="text-[var(--ink)]">Portfolio</span>
            <span className="hover:text-[var(--ink)]">Markets</span>
            <span className="hover:text-[var(--ink)]">Flow</span>
            <span className="hover:text-[var(--ink)]">GEX</span>
          </nav>

          <div className="ml-auto flex items-center gap-2">
            <button
              onClick={() => void load(true)}
              disabled={loading || refreshing}
              className="rounded-full border border-[var(--border)] px-3.5 py-1.5 text-[13px] font-medium text-[var(--text)] hover:bg-[var(--bg-soft)] disabled:opacity-50"
            >
              {refreshing ? "Refreshing…" : "Refresh"}
            </button>
            <span
              className={`rounded-full px-3 py-1.5 text-[12px] font-semibold ${
                live
                  ? "bg-emerald-50 text-emerald-700"
                  : "bg-amber-50 text-amber-700"
              }`}
            >
              {live
                ? cache?.hit
                  ? `Cached ${cache.age_secs}s`
                  : refreshing || loading
                    ? "Updating…"
                    : "Live UW"
                : loading
                  ? "…"
                  : "Offline"}
            </span>
            <button className="rounded-full bg-[var(--ink)] px-4 py-1.5 text-[13px] font-semibold text-white">
              Brokerage
            </button>
          </div>
        </header>

        {/* Ticker strip */}
        <div className="flex shrink-0 items-center gap-6 overflow-x-auto border-b border-[var(--border)] bg-white px-5 py-2.5 text-[13px]">
          {positions.length === 0 && (
            <span className="text-[var(--text-dim)]">
              {loading ? "Loading book…" : "No positions"}
            </span>
          )}
          {positions.map((p) => {
            const isPeak = peak?.id === p.id;
            return (
              <button
                key={p.id}
                onClick={() => {
                  setSelectedId(p.id);
                  setDetailTab("drivers");
                }}
                className={`flex shrink-0 items-center gap-2 whitespace-nowrap rounded-full px-2 py-0.5 ${
                  isPeak ? "bg-orange-50 ring-1 ring-orange-200" : ""
                }`}
              >
                <span className="font-semibold text-[var(--ink)]">{p.ticker}</span>
                <span
                  className="mono text-[12px] font-medium"
                  style={{ color: scoreBarHex(p.score) }}
                >
                  {p.score}
                </span>
                <span className={`text-[11px] ${bucketColor(p.bucket)}`}>
                  {bucketLabel(p.bucket)}
                </span>
                {isPeak && (
                  <span className="text-[9px] font-bold uppercase tracking-wide text-orange-600">
                    Peak
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {topWhy && (
          <div className="flex shrink-0 items-center gap-2 border-b border-orange-200/80 bg-[#fff7f0] px-5 py-2 text-[13px]">
            <span className="rounded-full bg-orange-100 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-orange-700">
              Top driver
            </span>
            <button
              type="button"
              className="truncate text-left font-medium text-[var(--ink)] hover:underline"
              onClick={() => {
                if (peak) {
                  setSelectedId(peak.id);
                  setDetailTab("drivers");
                  setMainTab("positions");
                }
              }}
            >
              {topWhy.ticker ? `${topWhy.ticker} — ` : ""}
              {topWhy.label}
            </button>
          </div>
        )}

        {error && (
          <div className="shrink-0 border-b border-rose-200 bg-rose-50 px-5 py-2 text-sm text-rose-700">
            {error}
          </div>
        )}

        {/* Main + right inbox */}
        <div className="grid min-h-0 flex-1 grid-cols-1 xl:grid-cols-[minmax(0,1fr)_360px]">
          <main className="min-h-0 overflow-auto px-5 py-6 md:px-8">
            <div className="mb-1 flex flex-wrap items-end gap-x-4 gap-y-2">
              <button
                type="button"
                disabled={!peak}
                onClick={() => {
                  if (!peak) return;
                  setSelectedId(peak.id);
                  setDetailTab("drivers");
                  setMainTab("positions");
                }}
                className="text-left disabled:cursor-default"
              >
                <h1
                  className="text-[44px] font-extrabold leading-none tracking-tight tabular-nums md:text-[56px]"
                  style={{
                    color: peak ? scoreBarHex(peak.score) : "var(--ink)",
                  }}
                >
                  {peak ? peak.score : loading ? "…" : "—"}
                </h1>
              </button>
              <div className="mb-1.5 flex flex-col gap-1.5">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-[15px] font-bold text-[var(--ink)]">
                    Peak Hazard
                    {peak ? (
                      <>
                        :{" "}
                        <span style={{ color: scoreBarHex(peak.score) }}>
                          {peak.ticker}
                        </span>{" "}
                        <span className={`${bucketColor(peak.bucket)}`}>
                          ({bucketLabel(peak.bucket)})
                        </span>
                      </>
                    ) : null}
                  </span>
                  <span className="rounded-full bg-[var(--accent-soft)] px-2.5 py-1 text-[11px] font-semibold text-[var(--accent)]">
                    Sample book
                  </span>
                </div>
                <div className="flex flex-wrap items-center gap-1.5 text-[13px] text-[var(--text-muted)]">
                  <span className="rounded-full border border-[var(--border)] bg-white px-2.5 py-0.5 font-medium text-[var(--ink)]">
                    {bookHealth.count} Positions
                  </span>
                  <span className="text-[var(--text-dim)]">|</span>
                  <span
                    className={`rounded-full border px-2.5 py-0.5 font-medium ${
                      bookHealth.critical > 0
                        ? "border-rose-200 bg-rose-50 text-rose-700"
                        : "border-[var(--border)] bg-white text-[var(--ink)]"
                    }`}
                  >
                    {bookHealth.critical} Critical
                  </span>
                  <span className="text-[var(--text-dim)]">|</span>
                  <span
                    className={`rounded-full border px-2.5 py-0.5 font-medium ${
                      bookHealth.elevated > 0
                        ? "border-orange-200 bg-orange-50 text-orange-700"
                        : "border-[var(--border)] bg-white text-[var(--ink)]"
                    }`}
                  >
                    {bookHealth.elevated} Elevated
                  </span>
                  {summary?.avg_risk_score != null && (
                    <>
                      <span className="text-[var(--text-dim)]">·</span>
                      <span className="text-[12px] text-[var(--text-dim)]">
                        book avg {summary.avg_risk_score}
                      </span>
                    </>
                  )}
                </div>
              </div>
            </div>

            <div className="mb-5 mt-5 flex items-center gap-5 border-b border-[var(--border)] text-[14px] font-medium">
              {(
                [
                  ["positions", "Positions"],
                  ["mix", "Scoring mix"],
                  ["probe", "UW Probe"],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  onClick={() => setMainTab(id)}
                  className={`relative pb-3 ${
                    mainTab === id
                      ? "text-[var(--ink)]"
                      : "text-[var(--text-muted)] hover:text-[var(--ink)]"
                  }`}
                >
                  {label}
                  {mainTab === id && (
                    <span className="absolute inset-x-0 -bottom-px h-0.5 bg-[var(--ink)]" />
                  )}
                </button>
              ))}
              <div className="ml-auto pb-3">
                <RiskBar score={peak?.score ?? 0} wide />
              </div>
            </div>

            {mainTab === "positions" && (
              <div className="overflow-visible rounded-xl border border-[var(--border)] bg-white">
                <div className="overflow-x-auto">
                  <table className="w-full min-w-[640px] text-left text-sm">
                    <thead className="bg-[var(--bg-soft)] text-[11px] uppercase tracking-[0.06em] text-[var(--text-dim)]">
                      <tr className="border-b border-[var(--border)]">
                        <th className="px-4 py-3 font-medium">Ticker</th>
                        <th className="px-3 py-3 font-medium">Contract</th>
                        <th className="px-3 py-3 font-medium">Spot</th>
                        <th className="px-3 py-3 font-medium">Risk</th>
                        <th className="px-3 py-3 font-medium">Bucket</th>
                        <th className="px-4 py-3 font-medium">Top Driver</th>
                      </tr>
                    </thead>
                    <tbody>
                      {loading && positions.length === 0 && (
                        <tr>
                          <td colSpan={6} className="px-4 py-16 text-center text-[var(--text-muted)]">
                            Scoring from live GEX + flow…
                          </td>
                        </tr>
                      )}
                      {filtered.map((p) => (
                        <PositionTableRow
                          key={p.id}
                          position={p}
                          active={selected?.id === p.id}
                          dimmed={refreshing}
                          onSelect={() => {
                            setSelectedId(p.id);
                            setDetailTab("drivers");
                          }}
                        />
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {mainTab === "mix" && (
              <div className="space-y-3">
                <p className="text-[14px] text-[var(--text-muted)]">
                  {summary?.methodology ??
                    "GEX (26) + flip (20) + walls (16) + flow (20) + structure (10) + earnings (8)."}
                </p>
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
                  {(methodology?.families ??
                    methodology?.mix?.map((m) => ({
                      family: m.family,
                      label: m.title,
                      max_points: m.max_points,
                    })) ??
                    defaultMix()
                  ).map((m) => (
                    <div
                      key={m.family}
                      className="flex items-center justify-between rounded-xl border border-[var(--border)] bg-white px-4 py-3"
                    >
                      <span className="text-[14px] font-medium text-[var(--ink)]">
                        {"label" in m ? m.label : m.family}
                      </span>
                      <span className="mono text-[13px] font-semibold text-[var(--accent)]">
                        max {m.max_points}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {mainTab === "probe" && (
              <div className="max-w-xl space-y-4 rounded-xl border border-[var(--border)] bg-white p-5">
                <div className="flex gap-2">
                  <input
                    value={probeTicker}
                    onChange={(e) => setProbeTicker(e.target.value.toUpperCase())}
                    className="w-28 rounded-full border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-1.5 mono text-sm outline-none focus:border-[var(--accent)]"
                  />
                  <button
                    onClick={() => void runProbe()}
                    disabled={probing}
                    className="rounded-full bg-[var(--ink)] px-4 py-1.5 text-sm font-semibold text-white disabled:opacity-50"
                  >
                    {probing ? "…" : "Probe"}
                  </button>
                  {selected && (
                    <button
                      onClick={() => {
                        setProbeTicker(selected.ticker);
                        void runProbe(selected.ticker);
                      }}
                      className="rounded-full border border-[var(--border)] px-3 py-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--ink)]"
                    >
                      Use {selected.ticker}
                    </button>
                  )}
                </div>
                {probe ? (
                  <>
                    <p className="text-sm text-[var(--text-muted)]">
                      {probe.info.full_name ?? probe.ticker}
                    </p>
                    <div className="grid grid-cols-2 gap-2">
                      <MiniStat label="Flip" value={probe.gex.gamma_flip ?? "—"} />
                      <MiniStat label="Call wall" value={probe.gex.call_wall ?? "—"} tone="up" />
                      <MiniStat label="Put wall" value={probe.gex.put_wall ?? "—"} tone="down" />
                      <MiniStat label="Magnet" value={probe.gex.gamma_magnet ?? "—"} />
                    </div>
                  </>
                ) : (
                  <p className="text-sm text-[var(--text-muted)]">
                    {probing ? "Fetching…" : "No probe data yet"}
                  </p>
                )}
              </div>
            )}
          </main>

          {/* Right inbox-style detail */}
          <aside className="hidden min-h-0 flex-col border-l border-[var(--border)] bg-white xl:flex">
            <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
              <h2 className="text-[15px] font-bold text-[var(--ink)]">Drivers</h2>
              <span className="text-[12px] text-[var(--text-dim)]">
                {selected ? selected.ticker : "—"}
              </span>
            </div>

            {!selected ? (
              <div className="flex flex-1 items-center justify-center px-4 text-sm text-[var(--text-muted)]">
                Select a position
              </div>
            ) : (
              <div className="min-h-0 flex-1 overflow-auto p-4">
                <div className="mb-4 rounded-xl bg-[var(--promo)] p-4 text-white">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <div className="text-[11px] font-medium uppercase tracking-wide text-white/60">
                        Position risk
                      </div>
                      <div className="mt-1 text-[28px] font-extrabold leading-none">
                        {selected.score}
                        <span className="text-[14px] font-medium text-white/50">
                          /{selected.score_max ?? 100}
                        </span>
                      </div>
                      <p className="mt-2 text-[12px] text-white/70">
                        {selected.side} {selected.quantity}× {selected.option_type}{" "}
                        {selected.strike}
                        {selected.underlying_price
                          ? ` · spot ${selected.underlying_price.toFixed(2)}`
                          : ""}
                      </p>
                    </div>
                    <span
                      className={`rounded-full border px-2 py-0.5 text-[11px] font-semibold ${bucketBg(selected.bucket)} ${bucketColor(selected.bucket)}`}
                    >
                      {bucketLabel(selected.bucket)}
                    </span>
                  </div>
                  <div className="mt-3">
                    <RiskBar score={selected.score} wide lightTrack />
                  </div>
                </div>

                {selected.gex_snapshot && (
                  <div className="mb-4 grid grid-cols-2 gap-2">
                    <MiniStat label="Flip" value={selected.gex_snapshot.gamma_flip ?? "—"} />
                    <MiniStat
                      label="Net GEX"
                      value={
                        selected.gex_snapshot.net_gamma != null
                          ? formatCompactNumber(selected.gex_snapshot.net_gamma)
                          : "—"
                      }
                      tone={
                        selected.gex_snapshot.positive_gamma_regime === false
                          ? "down"
                          : selected.gex_snapshot.positive_gamma_regime === true
                            ? "up"
                            : undefined
                      }
                    />
                    <MiniStat label="Call" value={selected.gex_snapshot.call_wall ?? "—"} tone="up" />
                    <MiniStat label="Put" value={selected.gex_snapshot.put_wall ?? "—"} tone="down" />
                  </div>
                )}

                <div className="mb-3 flex gap-1 rounded-full bg-[var(--bg-soft)] p-1">
                  {(
                    [
                      ["drivers", "Drivers"],
                      ["breakdown", "Break"],
                      ["probe", "Probe"],
                    ] as const
                  ).map(([id, label]) => (
                    <button
                      key={id}
                      onClick={() => setDetailTab(id)}
                      className={`flex-1 rounded-full px-2 py-1.5 text-[12px] font-semibold ${
                        detailTab === id
                          ? "bg-white text-[var(--ink)] shadow-sm"
                          : "text-[var(--text-muted)]"
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>

                {detailTab === "drivers" && (
                  <ul className="space-y-2">
                    {selected.drivers.map((d) => {
                      const max = d.max_points || 1;
                      const pct = Math.min(100, (d.contribution / max) * 100);
                      return (
                        <li
                          key={d.code}
                          className="rounded-xl border border-[var(--border)] bg-white p-3"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <span className="text-[13px] font-semibold text-[var(--ink)]">
                              {d.label}
                            </span>
                            <span className="mono shrink-0 text-[12px] font-bold text-[var(--accent)]">
                              {d.contribution.toFixed(1)}
                              <span className="font-medium text-[var(--text-dim)]">/{max}</span>
                            </span>
                          </div>
                          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-[var(--bg-soft)]">
                            <div
                              className="h-full rounded-full"
                              style={{ width: `${pct}%`, background: "var(--accent)" }}
                            />
                          </div>
                          {d.detail && (
                            <p className="mt-2 text-[12px] leading-relaxed text-[var(--text-muted)]">
                              {d.detail}
                            </p>
                          )}
                        </li>
                      );
                    })}
                  </ul>
                )}

                {detailTab === "breakdown" && (
                  <div className="space-y-3">
                    {(selected.family_scores && selected.family_scores.length > 0
                      ? selected.family_scores
                      : FALLBACK_FAMILIES.map((f) => ({
                          family: f.family,
                          label: f.label,
                          points:
                            (selected.breakdown?.[
                              f.family as keyof NonNullable<Position["breakdown"]>
                            ] as number) ?? 0,
                          max_points: f.max_points,
                        }))
                    ).map((row) => {
                      const pct = Math.min(100, (row.points / row.max_points) * 100);
                      return (
                        <div key={row.family}>
                          <div className="mb-1 flex justify-between text-[12px]">
                            <span className="text-[var(--text-muted)]">{row.label}</span>
                            <span className="mono font-semibold text-[var(--ink)]">
                              {row.points.toFixed(1)}
                              <span className="text-[var(--text-dim)]">/{row.max_points}</span>
                            </span>
                          </div>
                          <div className="h-2 overflow-hidden rounded-full bg-[var(--bg-soft)]">
                            <div
                              className="risk-bar-fill h-full rounded-full"
                              style={{
                                width: `${pct}%`,
                                background: row.points <= 0 ? "transparent" : "var(--accent)",
                              }}
                            />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}

                {detailTab === "probe" && (
                  <div className="space-y-3">
                    <button
                      onClick={() => {
                        setMainTab("probe");
                        void runProbe(selected.ticker);
                      }}
                      className="w-full rounded-full border border-[var(--border)] py-2 text-[13px] font-semibold text-[var(--ink)] hover:bg-[var(--bg-soft)]"
                    >
                      Open full probe for {selected.ticker}
                    </button>
                    {(probe?.recent_alerts ?? []).slice(0, 4).map((a, i) => {
                      const t = a.option_type ?? a.type ?? "?";
                      return (
                        <div
                          key={`${a.created_at}-${i}`}
                          className="rounded-xl border border-[var(--border)] px-3 py-2.5"
                        >
                          <div className="flex justify-between text-[12px]">
                            <span className="font-semibold">
                              <span className={t === "put" ? "text-[var(--put)]" : "text-[var(--call)]"}>
                                {t}
                              </span>{" "}
                              {a.strike}
                            </span>
                            <span className="text-[var(--text-dim)]">{a.alert_rule}</span>
                          </div>
                          <p className="mt-1 text-[12px] text-[var(--accent)]">
                            ${a.total_premium} premium
                          </p>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </aside>
        </div>
      </div>
    </div>
  );
}

function familyRowsFor(p: Position): FamilyScore[] {
  if (p.family_scores && p.family_scores.length > 0) return p.family_scores;
  return FALLBACK_FAMILIES.map((f) => ({
    family: f.family,
    label: f.label,
    points:
      (p.breakdown?.[f.family as keyof NonNullable<Position["breakdown"]>] as number) ?? 0,
    max_points: f.max_points,
  }));
}

function PositionTableRow({
  position: p,
  active,
  dimmed,
  onSelect,
}: {
  position: Position;
  active: boolean;
  dimmed: boolean;
  onSelect: () => void;
}) {
  const rowRef = useRef<HTMLTableRowElement>(null);
  const leaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState({ top: 0, left: 0 });

  function clearLeave() {
    if (leaveTimer.current) {
      clearTimeout(leaveTimer.current);
      leaveTimer.current = null;
    }
  }

  function show() {
    clearLeave();
    const el = rowRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const width = 320;
    const left = Math.min(
      Math.max(12, r.right - width),
      window.innerWidth - width - 12,
    );
    const estimatedH = 280;
    const top =
      r.bottom + 8 + estimatedH > window.innerHeight
        ? Math.max(12, r.top - estimatedH - 8)
        : r.bottom + 8;
    setCoords({ top, left });
    setOpen(true);
  }

  function scheduleHide() {
    clearLeave();
    leaveTimer.current = setTimeout(() => setOpen(false), 120);
  }

  useEffect(() => () => clearLeave(), []);

  const families = familyRowsFor(p);

  return (
    <>
      <tr
        ref={rowRef}
        onClick={onSelect}
        onMouseEnter={show}
        onMouseLeave={scheduleHide}
        className={`cursor-pointer border-b border-[var(--border)] last:border-0 ${
          active ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--bg-soft)]"
        } ${dimmed ? "opacity-80" : ""}`}
      >
        <td className="px-4 py-3.5">
          <span className="font-semibold text-[var(--ink)]">{p.ticker}</span>
          <div className="mono text-[11px] text-[var(--text-dim)]">
            Δ {p.delta.toFixed(2)} · Γ {p.gamma.toFixed(3)}
          </div>
        </td>
        <td className="px-3 py-3.5 mono text-[13px]">
          <span className={p.option_type === "put" ? "text-[var(--put)]" : "text-[var(--call)]"}>
            {p.option_type}
          </span>{" "}
          {p.strike}
          <span className="text-[var(--text-dim)]"> · {p.expiry.slice(5)}</span>
        </td>
        <td className="px-3 py-3.5 mono text-[13px] text-[var(--text-muted)]">
          {p.underlying_price ? p.underlying_price.toFixed(2) : "—"}
        </td>
        <td className="px-3 py-3.5">
          <div className="flex items-center gap-2">
            <span
              className="mono w-6 text-right text-sm font-bold"
              style={{ color: scoreBarHex(p.score) }}
            >
              {p.score}
            </span>
            <RiskBar score={p.score} />
          </div>
        </td>
        <td className="px-3 py-3.5">
          <span
            className={`inline-flex rounded-full border px-2 py-0.5 text-[11px] font-semibold ${bucketBg(p.bucket)} ${bucketColor(p.bucket)}`}
          >
            {bucketLabel(p.bucket)}
          </span>
        </td>
        <td className="max-w-[220px] truncate px-4 py-3.5 text-[13px] text-[var(--text-muted)]">
          {p.drivers[0]?.label ?? "—"}
        </td>
      </tr>
      {open &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            role="tooltip"
            onMouseEnter={clearLeave}
            onMouseLeave={scheduleHide}
            className="fixed z-[80] w-[320px] rounded-xl border border-[var(--border)] bg-white p-3.5 shadow-[0_12px_40px_rgba(15,15,20,0.12)]"
            style={{ top: coords.top, left: coords.left }}
          >
            <div className="mb-2.5 flex items-start justify-between gap-2">
              <div>
                <div className="text-[14px] font-bold text-[var(--ink)]">
                  {p.ticker}{" "}
                  <span className="font-medium text-[var(--text-muted)]">breakdown</span>
                </div>
                <div className="mono text-[11px] text-[var(--text-dim)]">
                  {p.option_type} {p.strike} · {bucketLabel(p.bucket)}
                </div>
              </div>
              <div className="text-right">
                <div
                  className="mono text-[20px] font-extrabold leading-none"
                  style={{ color: scoreBarHex(p.score) }}
                >
                  {p.score}
                </div>
                <div className="mono text-[10px] text-[var(--text-dim)]">
                  / {p.score_max ?? 100}
                </div>
              </div>
            </div>
            <ul className="space-y-2">
              {families.map((row) => {
                const pct = Math.min(100, (row.points / row.max_points) * 100);
                return (
                  <li key={row.family}>
                    <div className="mb-0.5 flex justify-between gap-2 text-[11px]">
                      <span className="truncate text-[var(--text-muted)]">{row.label}</span>
                      <span className="mono shrink-0 font-semibold text-[var(--ink)]">
                        {row.points.toFixed(1)}
                        <span className="font-medium text-[var(--text-dim)]">/{row.max_points}</span>
                      </span>
                    </div>
                    <div className="h-1.5 overflow-hidden rounded-full bg-[var(--bg-soft)]">
                      <div
                        className="h-full rounded-full"
                        style={{
                          width: `${pct}%`,
                          background: row.points <= 0 ? "transparent" : scoreBarHex(
                            (row.points / row.max_points) * 100,
                          ),
                        }}
                      />
                    </div>
                  </li>
                );
              })}
            </ul>
            {p.drivers[0] && (
              <p className="mt-2.5 border-t border-[var(--border)] pt-2 text-[11px] leading-snug text-[var(--text-muted)]">
                <span className="font-semibold text-[var(--ink)]">Top: </span>
                {p.drivers[0].label}
                {p.drivers[0].detail ? ` — ${p.drivers[0].detail.slice(0, 110)}` : ""}
              </p>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

function RiskBar({
  score,
  wide,
  lightTrack,
}: {
  score: number;
  wide?: boolean;
  lightTrack?: boolean;
}) {
  const color = scoreBarHex(score);
  return (
    <div
      className={`overflow-hidden rounded-full ${
        lightTrack ? "bg-white/20" : "bg-zinc-100"
      } ${wide ? "h-2 w-full max-w-[180px]" : "h-1.5 w-[72px]"}`}
    >
      <div
        className="risk-bar-fill h-full rounded-full"
        style={{
          width: `${Math.max(4, Math.min(100, score))}%`,
          background: color,
        }}
      />
    </div>
  );
}

function MiniStat({
  label,
  value,
  tone,
  title,
}: {
  label: string;
  value: string;
  tone?: "up" | "down";
  title?: string;
}) {
  return (
    <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2" title={title}>
      <div className="text-[10px] font-medium uppercase tracking-wide text-[var(--text-dim)]">
        {label}
      </div>
      <div
        className={`mono mt-0.5 text-[13px] font-bold ${
          tone === "up"
            ? "text-[var(--up)]"
            : tone === "down"
              ? "text-[var(--down)]"
              : "text-[var(--ink)]"
        }`}
      >
        {value}
      </div>
    </div>
  );
}

function defaultMix() {
  return [
    { family: "gamma_regime", label: "Dealer GEX", max_points: 26 },
    { family: "flip_proximity", label: "Flip proximity", max_points: 20 },
    { family: "wall_conflict", label: "Wall conflict", max_points: 16 },
    { family: "adverse_flow", label: "Adverse flow", max_points: 20 },
    { family: "structure", label: "Structure", max_points: 10 },
    { family: "catalyst", label: "Earnings", max_points: 8 },
  ];
}
