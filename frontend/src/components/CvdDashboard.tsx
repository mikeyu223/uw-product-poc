"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchCvdProfile, fetchFuturesContracts, fetchHealth, fetchOiMatrix } from "@/lib/api";
import type {
  CvdProfile,
  FuturesContract,
  FuturesGex,
  GexHeatmap,
  OiMatrixReport,
  PriceLevelDelta,
  SessionLevels,
  VolumeProfile,
} from "@/lib/types";

/** Fast poll — tape should feel closer to Live Options Flow than a dashboard refresh. */
const POLL_MS = 2_500;

export function CvdDashboard({ embedded = false }: { embedded?: boolean }) {
  const [backendOk, setBackendOk] = useState<boolean | null>(null);
  const [contracts, setContracts] = useState<FuturesContract[]>([]);
  const [contract, setContract] = useState<string>("");
  const [profile, setProfile] = useState<CvdProfile | null>(null);
  const [limit, setLimit] = useState(500);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<"all" | "large" | "blocks">("all");
  const [rail, setRail] = useState<"flow" | "gex" | "levels">("gex");
  const [oi, setOi] = useState<OiMatrixReport | null>(null);
  const seenIds = useRef<Set<string>>(new Set());
  const [flashIds, setFlashIds] = useState<Set<string>>(new Set());

  const loadContracts = useCallback(async () => {
    const health = await fetchHealth();
    setBackendOk(health.status === "ok");
    const [list, oiReport] = await Promise.all([
      fetchFuturesContracts(),
      fetchOiMatrix().catch(() => null),
    ]);
    setContracts(list);
    if (oiReport) setOi(oiReport);
    setContract((prev) => {
      if (prev && list.some((c) => c.name === prev)) return prev;
      return list.find((c) => c.product === "ES")?.name ?? list[0]?.name ?? "";
    });
  }, []);

  const loadProfile = useCallback(
    async (silent = false) => {
      if (!contract) return;
      if (!silent) setLoading(true);
      else setRefreshing(true);
      try {
        const p = await fetchCvdProfile(contract, limit);
        // Flash new tape rows (newest-first).
        const nextFlash = new Set<string>();
        for (const row of p.tape.slice(0, 40)) {
          const id = String(row.trade_id ?? `${row.t}-${row.size}-${row.price}`);
          if (seenIds.current.size > 0 && !seenIds.current.has(id)) {
            nextFlash.add(id);
          }
          seenIds.current.add(id);
        }
        if (nextFlash.size) {
          setFlashIds(nextFlash);
          setTimeout(() => setFlashIds(new Set()), 900);
        }
        setProfile(p);
        setBackendOk(true);
        setError(null);
      } catch (e) {
        setBackendOk(false);
        if (!silent) setError(e instanceof Error ? e.message : "Failed to load tape");
      } finally {
        setLoading(false);
        setRefreshing(false);
      }
    },
    [contract, limit],
  );

  useEffect(() => {
    void (async () => {
      try {
        await loadContracts();
      } catch (e) {
        setBackendOk(false);
        setError(e instanceof Error ? e.message : "Failed to load contracts");
        setLoading(false);
      }
    })();
  }, [loadContracts]);

  useEffect(() => {
    seenIds.current = new Set();
    if (!contract) return;
    void loadProfile(false);
  }, [contract, loadProfile]);

  useEffect(() => {
    if (!contract) return;
    const id = setInterval(() => void loadProfile(true), POLL_MS);
    return () => clearInterval(id);
  }, [contract, loadProfile]);

  const tapeRows = useMemo(() => {
    if (!profile) return [];
    if (filter === "large") return profile.tape.filter((t) => t.is_large || t.is_block);
    if (filter === "blocks") return profile.tape.filter((t) => t.is_block);
    return profile.tape;
  }, [profile, filter]);

  const s = profile?.summary;
  const flow = profile?.order_flow;

  return (
    <div
      className={`flex flex-col overflow-hidden bg-[var(--bg-soft)] ${
        embedded ? "h-full" : "h-[100dvh]"
      }`}
    >
      {/* Top bar — product, not marketing */}
      <header className="shrink-0 border-b border-[var(--border)] bg-white px-3 py-2.5 sm:px-5">
        <div className="flex flex-wrap items-center gap-2 sm:gap-3">
          <div className="shrink-0">
            <div className="text-[15px] font-extrabold tracking-tight text-[var(--ink)] sm:text-[17px]">
              futures<span className="text-[var(--accent)]">tape</span>
            </div>
            <div className="hidden text-[10px] font-medium uppercase tracking-[0.12em] text-[var(--text-dim)] sm:block">
              Who’s hitting · dealer gamma · key levels
            </div>
          </div>

          <div className="flex items-center gap-1">
            {contracts.map((c) => {
              const primary = c.product === "ES" || c.product === "NQ";
              return (
                <button
                  key={c.name}
                  type="button"
                  onClick={() => setContract(c.name)}
                  className={`rounded-full px-2.5 py-1 text-[12px] font-bold transition-colors sm:px-3 sm:py-1.5 ${
                    contract === c.name
                      ? "bg-[var(--ink)] text-white"
                      : primary
                        ? "border border-[var(--border-strong)] bg-white text-[var(--ink)]"
                        : "border border-[var(--border)] bg-white text-[var(--text-muted)]"
                  }`}
                >
                  {c.product}
                </button>
              );
            })}
          </div>

          <div className="ml-auto flex items-center gap-2">
            <span className="hidden text-[11px] text-[var(--text-dim)] md:inline">
              {profile?.contract}
              {refreshing ? " · updating" : ""}
            </span>
            <button
              type="button"
              onClick={() => setRail("gex")}
              className={`rounded-full px-3 py-1.5 text-[12px] font-bold transition-colors ${
                rail === "gex"
                  ? "bg-violet-600 text-white shadow-sm"
                  : "border border-violet-300 bg-violet-50 text-violet-800 hover:bg-violet-100"
              }`}
              title="Open dealer GEX heatmap"
            >
              GEX Heatmap
            </button>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="rounded-full border border-[var(--border)] bg-[var(--bg-soft)] px-2.5 py-1 text-[11px] outline-none"
            >
              <option value={200}>200</option>
              <option value={500}>500</option>
              <option value={1000}>1000</option>
            </select>
            <span
              className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold ${
                backendOk
                  ? "bg-emerald-50 text-emerald-700"
                  : "bg-rose-50 text-rose-700"
              }`}
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  backendOk ? "bg-emerald-500 animate-pulse" : "bg-rose-500"
                }`}
              />
              {backendOk ? "LIVE" : "OFF"}
            </span>
          </div>
        </div>
      </header>

      {error && (
        <div className="shrink-0 border-b border-rose-200 bg-rose-50 px-4 py-2 text-sm text-rose-700">
          {error}
        </div>
      )}

      {loading && !profile ? (
        <div className="flex flex-1 items-center justify-center text-[var(--text-muted)]">
          Wiring live futures tape…
        </div>
      ) : profile && s && flow ? (
        <div className="mx-auto grid min-h-0 w-full max-w-[1700px] flex-1 grid-cols-1 gap-0 lg:grid-cols-[minmax(0,1fr)_minmax(380px,1.05fr)]">
          {/* LEFT — Live Tape */}
          <section className="flex min-h-0 flex-col border-r border-[var(--border)] bg-white">
            <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-[var(--border)] px-3 py-2.5 sm:px-4">
              <div>
                <h2 className="text-[13px] font-bold text-[var(--ink)]">Live tape</h2>
                <p className="text-[11px] text-[var(--text-dim)]">
                  Who hit the market · {profile.product}
                </p>
              </div>
              <div className="ml-auto flex gap-1">
                {(
                  [
                    ["all", "All"],
                    ["large", "Large"],
                    ["blocks", "Blocks"],
                  ] as const
                ).map(([id, label]) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setFilter(id)}
                    className={`rounded-full px-2.5 py-0.5 text-[11px] font-semibold ${
                      filter === id
                        ? "bg-[var(--ink)] text-white"
                        : "text-[var(--text-muted)] hover:bg-[var(--bg-soft)]"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>

            <div className="min-h-0 flex-1 overflow-auto">
              <table className="w-full min-w-[560px] text-left text-[12px]">
                <thead className="sticky top-0 z-10 border-b border-[var(--border)] bg-[var(--bg-soft)] text-[10px] uppercase tracking-[0.08em] text-[var(--text-dim)]">
                  <tr>
                    <th className="px-3 py-2 font-semibold">Time</th>
                    <th className="px-2 py-2 font-semibold">Side</th>
                    <th className="px-2 py-2 font-semibold">Size</th>
                    <th className="px-2 py-2 font-semibold">Price</th>
                    <th className="px-2 py-2 font-semibold">CVD</th>
                    <th className="px-3 py-2 font-semibold">Flags</th>
                  </tr>
                </thead>
                <tbody>
                  {tapeRows.slice(0, 200).map((row) => {
                    const id = String(row.trade_id ?? `${row.t}-${row.size}-${row.price}`);
                    const buy = row.side === "buy";
                    const flash = flashIds.has(id);
                    return (
                      <tr
                        key={id}
                        className={`border-b border-[var(--border)]/70 ${
                          flash
                            ? "bg-amber-50"
                            : row.is_block
                              ? "bg-[#fff8f0]"
                              : row.is_large
                                ? "bg-[var(--bg-soft)]/80"
                                : "bg-white"
                        }`}
                      >
                        <td className="mono whitespace-nowrap px-3 py-1.5 text-[var(--text-dim)]">
                          {formatTime(row.t)}
                        </td>
                        <td className="px-2 py-1.5">
                          <span
                            className={`rounded px-1.5 py-0.5 text-[10px] font-bold uppercase ${
                              buy
                                ? "bg-emerald-50 text-emerald-700"
                                : "bg-rose-50 text-rose-700"
                            }`}
                          >
                            {buy ? "Buy" : "Sell"}
                          </span>
                        </td>
                        <td
                          className={`mono px-2 py-1.5 font-semibold ${
                            row.is_large || row.is_block ? "text-[var(--ink)]" : "text-[var(--text)]"
                          }`}
                        >
                          {fmtInt(row.size)}
                        </td>
                        <td className="mono px-2 py-1.5 text-[var(--ink)]">
                          {row.price_display}
                        </td>
                        <td
                          className="mono px-2 py-1.5 font-medium"
                          style={{ color: row.cvd >= 0 ? "#16a34a" : "#e11d48" }}
                        >
                          {fmtSigned(row.cvd)}
                        </td>
                        <td className="px-3 py-1.5">
                          <div className="flex flex-wrap gap-1">
                            {row.is_block && (
                              <span className="rounded bg-orange-100 px-1.5 py-0.5 text-[9px] font-bold uppercase text-orange-800">
                                Block
                              </span>
                            )}
                            {row.is_large && !row.is_block && (
                              <span className="rounded bg-sky-100 px-1.5 py-0.5 text-[9px] font-bold uppercase text-sky-800">
                                Large
                              </span>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </section>

          {/* RIGHT — tabbed context */}
          <section className="flex min-h-0 flex-col bg-[var(--bg-soft)] lg:overflow-hidden">
            <StatusStrip profile={profile} />

            <div className="flex shrink-0 gap-1 border-b border-[var(--border)] bg-white px-3 py-2">
              {(
                [
                  ["flow", "Order flow", "CVD & aggression"],
                  ["gex", "Dealer GEX", "Flip · walls · heatmap"],
                  ["levels", "Key levels", "POC · Globex · OI"],
                ] as const
              ).map(([id, label, hint]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setRail(id)}
                  className={`rounded-lg px-3 py-1.5 text-left transition-colors ${
                    rail === id
                      ? "bg-[var(--ink)] text-white"
                      : "text-[var(--text-muted)] hover:bg-[var(--bg-soft)]"
                  }`}
                >
                  <div className="text-[12px] font-bold">{label}</div>
                  <div
                    className={`text-[10px] ${
                      rail === id ? "text-white/55" : "text-[var(--text-dim)]"
                    }`}
                  >
                    {hint}
                  </div>
                </button>
              ))}
            </div>

            <div className="min-h-0 flex-1 overflow-auto bg-white">
              {rail === "flow" && (
                <div className="space-y-0">
                  <div className="border-b border-[var(--border)] bg-[var(--promo)] px-4 py-4 text-white">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-white/45">
                          Cumulative Volume Delta
                        </div>
                        <div className="mt-0.5 text-[12px] text-white/60">
                          Running score of aggressive buys vs sells
                        </div>
                      </div>
                      <div
                        className={`rounded-full px-2.5 py-1 text-[11px] font-bold ${
                          s.bias === "buyers"
                            ? "bg-emerald-400/20 text-emerald-300"
                            : s.bias === "sellers"
                              ? "bg-rose-400/20 text-rose-300"
                              : "bg-white/10 text-white/70"
                        }`}
                      >
                        {s.bias === "buyers"
                          ? "Buyers in control"
                          : s.bias === "sellers"
                            ? "Sellers in control"
                            : "Two-way"}
                      </div>
                    </div>
                    <div className="mt-2 flex items-end gap-3">
                      <div
                        className="text-[40px] font-extrabold leading-none tabular-nums"
                        style={{ color: s.cvd_last >= 0 ? "#4ade80" : "#fb7185" }}
                      >
                        {fmtSigned(s.cvd_last)}
                      </div>
                      <div className="mb-1 text-[12px] text-white/55">
                        net {fmtSigned(s.net_delta)} contracts
                      </div>
                    </div>
                    <div className="mt-3">
                      <div className="mb-1 flex justify-between text-[10px] font-semibold uppercase tracking-wide text-white/45">
                        <span>Buy {flow.buy_pct.toFixed(0)}%</span>
                        <span>Sell {flow.sell_pct.toFixed(0)}%</span>
                      </div>
                      <div className="flex h-2.5 overflow-hidden rounded-full bg-white/10">
                        <div className="h-full bg-emerald-400" style={{ width: `${flow.buy_pct}%` }} />
                        <div className="h-full bg-rose-400" style={{ width: `${flow.sell_pct}%` }} />
                      </div>
                    </div>
                    <div className="mt-3 grid grid-cols-3 gap-2 text-[11px]">
                      <Mini label="Large prints" value={String(flow.large_prints)} />
                      <Mini label="Blocks" value={String(flow.block_prints)} />
                      <Mini label="Max size" value={fmtInt(flow.max_print_size)} />
                    </div>
                  </div>
                  <div className="border-b border-[var(--border)] px-4 py-3">
                    <div className="mb-1 flex items-baseline justify-between">
                      <div className="text-[12px] font-bold text-[var(--ink)]">CVD trend</div>
                      <div className="text-[10px] text-[var(--text-dim)]">Session window</div>
                    </div>
                    <CvdChart series={profile.series} />
                  </div>
                  <div className="px-4 py-3">
                    <div className="mb-1 flex items-baseline justify-between">
                      <div className="text-[12px] font-bold text-[var(--ink)]">
                        Aggression by price
                      </div>
                      <div className="text-[10px] text-[var(--text-dim)]">
                        Green = buy heavy · Red = sell heavy
                      </div>
                    </div>
                    <PriceProfile levels={profile.levels} tick={s.tick_size} />
                  </div>
                </div>
              )}

              {rail === "gex" && (
                <div className="px-4 py-3">
                  {profile.futures_gex ? (
                    <FuturesGexPanel gex={profile.futures_gex} />
                  ) : (
                    <p className="text-[12px] text-[var(--text-dim)]">Loading dealer GEX…</p>
                  )}
                </div>
              )}

              {rail === "levels" && (
                <div className="space-y-5 px-4 py-3">
                  {profile.volume_profile && (
                    <VolumeProfilePanel vp={profile.volume_profile} />
                  )}
                  {profile.sessions && <SessionPanel sessions={profile.sessions} />}
                  {oi && <OiPanel report={oi} product={profile.product} />}
                </div>
              )}
            </div>
          </section>
        </div>
      ) : null}
    </div>
  );
}

function CvdChart({ series }: { series: CvdProfile["series"] }) {
  const path = useMemo(() => {
    if (series.length < 2) return null;
    const w = 520;
    const h = 140;
    const pad = 8;
    const ys = series.map((p) => p.cvd);
    const minY = Math.min(...ys, 0);
    const maxY = Math.max(...ys, 0);
    const span = Math.max(1, maxY - minY);
    const pts = series.map((p, i) => {
      const x = pad + (i / (series.length - 1)) * (w - pad * 2);
      const y = pad + (1 - (p.cvd - minY) / span) * (h - pad * 2);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    const zeroY = pad + (1 - (0 - minY) / span) * (h - pad * 2);
    const last = series[series.length - 1]?.cvd ?? 0;
    return { d: `M ${pts.join(" L ")}`, zeroY, last, w, h };
  }, [series]);

  if (!path) {
    return <div className="h-[140px] text-[12px] text-[var(--text-dim)]">Waiting for prints…</div>;
  }

  return (
    <svg viewBox={`0 0 ${path.w} ${path.h}`} className="h-[140px] w-full">
      <line
        x1={8}
        x2={path.w - 8}
        y1={path.zeroY}
        y2={path.zeroY}
        stroke="#e8e8ea"
        strokeDasharray="3 3"
      />
      <path
        d={path.d}
        fill="none"
        stroke={path.last >= 0 ? "#16a34a" : "#e11d48"}
        strokeWidth={2}
        strokeLinejoin="round"
      />
    </svg>
  );
}

function PriceProfile({ levels, tick }: { levels: PriceLevelDelta[]; tick: number }) {
  const rows = levels.slice(0, 22);
  const maxAbs = Math.max(1, ...rows.map((l) => Math.abs(l.net_delta)));
  if (!rows.length) {
    return <div className="text-[12px] text-[var(--text-dim)]">No levels yet</div>;
  }
  return (
    <div className="max-h-[240px] space-y-1 overflow-auto">
      {rows.map((l) => {
        const pct = (Math.abs(l.net_delta) / maxAbs) * 100;
        const buy = l.net_delta >= 0;
        return (
          <div
            key={l.price}
            className="grid grid-cols-[64px_1fr_48px] items-center gap-2 text-[11px]"
          >
            <span className="mono text-right text-[var(--text-muted)]">
              {l.price.toFixed(tick < 1 ? 2 : 0)}
            </span>
            <div className="relative h-3.5 overflow-hidden rounded bg-[var(--bg-soft)]">
              <div
                className="absolute inset-y-0 rounded"
                style={{
                  width: `${pct}%`,
                  [buy ? "left" : "right"]: 0,
                  background: buy ? "rgba(22,163,74,0.7)" : "rgba(225,29,72,0.7)",
                }}
              />
            </div>
            <span
              className="mono text-right font-semibold"
              style={{ color: buy ? "#16a34a" : "#e11d48" }}
            >
              {fmtSigned(l.net_delta)}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function Mini({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-white/10 px-2 py-1.5">
      <div className="text-[9px] uppercase tracking-wide text-white/45">{label}</div>
      <div className="mono text-[14px] font-bold">{value}</div>
    </div>
  );
}

function StatusStrip({ profile }: { profile: CvdProfile }) {
  const gex = profile.futures_gex;
  const bias = profile.summary.bias;
  const flowLabel =
    bias === "buyers" ? "Buyers hitting" : bias === "sellers" ? "Sellers hitting" : "Two-way tape";
  const gexLabel = !gex
    ? null
    : gex.regime.includes("negative")
      ? "Volatile gamma"
      : gex.regime.includes("positive")
        ? "Calm gamma"
        : gex.position_vs_flip === "above_flip"
          ? "Above flip"
          : gex.position_vs_flip === "below_flip"
            ? "Below flip"
            : null;

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-[var(--border)] bg-white px-3 py-2">
      <span className="text-[10px] font-semibold uppercase tracking-wide text-[var(--text-dim)]">
        Now
      </span>
      <span
        className={`rounded-full px-2.5 py-1 text-[11px] font-bold ${
          bias === "buyers"
            ? "bg-emerald-50 text-emerald-700"
            : bias === "sellers"
              ? "bg-rose-50 text-rose-700"
              : "bg-[var(--bg-soft)] text-[var(--text-muted)]"
        }`}
      >
        {flowLabel}
      </span>
      {gexLabel && (
        <span
          className={`rounded-full px-2.5 py-1 text-[11px] font-bold ${
            gexLabel.includes("Volatile") || gexLabel.includes("Below")
              ? "bg-orange-50 text-orange-800"
              : "bg-sky-50 text-sky-800"
          }`}
        >
          {gexLabel}
        </span>
      )}
      {gex?.zero_gamma_flip != null && profile.summary.last_price != null && (
        <span className="mono text-[11px] text-[var(--text-muted)]">
          last {profile.summary.last_price.toFixed(2)} · flip {gex.zero_gamma_flip.toFixed(0)}
        </span>
      )}
    </div>
  );
}

function FuturesGexPanel({ gex }: { gex: FuturesGex }) {
  if (!gex.proxy_ticker && gex.regime === "unknown") {
    return (
      <div className="text-[12px] text-[var(--text-dim)]">Loading dealer gamma…</div>
    );
  }

  const volatile =
    gex.regime.includes("negative") || gex.regime === "below_zero_gamma";
  const plain =
    volatile
      ? "Dealers are short gamma — moves can accelerate. Respect breakouts."
      : gex.regime.includes("positive") || gex.regime === "above_zero_gamma"
        ? "Dealers are long gamma — expect mean-reversion / pin near walls."
        : "Watch the flip level — regime can change with a few handles.";

  return (
    <div className="space-y-3">
      <div>
        <div className="flex items-baseline justify-between gap-2">
          <div className="text-[13px] font-bold text-[var(--ink)]">Dealer GEX</div>
          <div className="text-[10px] text-[var(--text-dim)]">
            {gex.product} via {gex.proxy_ticker} options
          </div>
        </div>
        <p className="mt-1 text-[12px] leading-snug text-[var(--text-muted)]">{plain}</p>
      </div>

      <div
        className={`rounded-xl border px-3 py-2.5 ${
          volatile
            ? "border-orange-200 bg-orange-50"
            : "border-emerald-200 bg-emerald-50"
        }`}
      >
        <div
          className={`text-[11px] font-bold uppercase tracking-wide ${
            volatile ? "text-orange-800" : "text-emerald-800"
          }`}
        >
          {volatile ? "Volatile regime" : "Mean-reverting regime"}
        </div>
        <div className="mt-0.5 text-[12px] text-[var(--ink)]">{gex.regime_label}</div>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <WatchLevel
          label="Zero-γ flip"
          hint="Regime line"
          value={gex.zero_gamma_flip}
          tone="violet"
        />
        <WatchLevel
          label="Call wall"
          hint="Resistance"
          value={gex.call_wall}
          tone="emerald"
        />
        <WatchLevel
          label="Put wall"
          hint="Support"
          value={gex.put_wall}
          tone="rose"
        />
        <WatchLevel
          label="Dist to flip"
          hint={gex.position_vs_flip.replace(/_/g, " ")}
          value={gex.distance_to_flip}
          tone="ink"
          signed
        />
      </div>

      <div>
        <div className="mb-1.5 flex items-baseline justify-between">
          <div className="text-[12px] font-bold text-[var(--ink)]">Gamma heatmap</div>
          <div className="text-[10px] text-[var(--text-dim)]">
            Green = +γ · Red = −γ · Blue row = spot
          </div>
        </div>
        {gex.heatmap && gex.heatmap.rows.length > 0 ? (
          <GexHeatmapGrid heatmap={gex.heatmap} gex={gex} />
        ) : (
          <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-6 text-center text-[12px] text-[var(--text-dim)]">
            Heatmap loading from UW greek exposure…
          </div>
        )}
      </div>
    </div>
  );
}

function WatchLevel({
  label,
  hint,
  value,
  tone,
  signed = false,
}: {
  label: string;
  hint: string;
  value?: number | null;
  tone: "violet" | "emerald" | "rose" | "ink";
  signed?: boolean;
}) {
  const accent =
    tone === "violet"
      ? "#7c3aed"
      : tone === "emerald"
        ? "#059669"
        : tone === "rose"
          ? "#e11d48"
          : "#111113";
  const display =
    value == null
      ? "—"
      : signed
        ? `${value >= 0 ? "+" : ""}${value.toFixed(1)}`
        : value.toFixed(value % 1 === 0 ? 0 : 2);
  return (
    <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-soft)] px-2.5 py-2">
      <div className="text-[9px] font-semibold uppercase tracking-wide" style={{ color: accent }}>
        {label}
      </div>
      <div className="mono text-[16px] font-extrabold text-[var(--ink)]">{display}</div>
      <div className="text-[10px] text-[var(--text-dim)]">{hint}</div>
    </div>
  );
}

function GexHeatmapGrid({
  heatmap,
  gex,
}: {
  heatmap: GexHeatmap;
  gex: FuturesGex;
}) {
  const cols = heatmap.expiries;
  const spot = gex.futures_last ?? heatmap.spot ?? null;

  return (
    <div className="overflow-auto rounded-lg border border-[var(--border)] bg-[#0f1419]">
      <div
        className="min-w-[520px] text-[9px]"
        style={{
          display: "grid",
          gridTemplateColumns: `56px repeat(${cols.length}, minmax(44px, 1fr)) 52px`,
        }}
      >
        {/* header */}
        <div className="sticky top-0 z-[1] border-b border-white/10 bg-[#0f1419] px-1 py-1.5 font-semibold text-[#8b93a7]">
          Strike
        </div>
        {cols.map((c) => (
          <div
            key={c.expiry}
            className="sticky top-0 z-[1] border-b border-white/10 bg-[#0f1419] px-0.5 py-1.5 text-center font-semibold text-[#8b93a7]"
            title={c.expiry}
          >
            {c.label}
          </div>
        ))}
        <div className="sticky top-0 z-[1] border-b border-white/10 bg-[#0f1419] px-1 py-1.5 text-center font-semibold text-[#8b93a7]">
          Σ
        </div>

        {heatmap.rows.map((row) => {
          const step =
            heatmap.rows.length > 1
              ? Math.abs(heatmap.rows[0].strike - heatmap.rows[1].strike)
              : 5;
          const isSpot = spot != null && Math.abs(row.strike - spot) <= step / 2 + 0.01;
          const isCallWall =
            gex.call_wall != null && Math.abs(row.strike - gex.call_wall) < 3;
          const isPutWall =
            gex.put_wall != null && Math.abs(row.strike - gex.put_wall) < 3;
          const isFlip =
            gex.zero_gamma_flip != null && Math.abs(row.strike - gex.zero_gamma_flip) < 3;
          const isMoveHigh =
            heatmap.move_high != null && Math.abs(row.strike - heatmap.move_high) < 4;
          const isMoveLow =
            heatmap.move_low != null && Math.abs(row.strike - heatmap.move_low) < 4;

          return (
            <div key={row.strike} className="contents">
              <div
                className={`mono flex items-center border-b border-white/5 px-1 py-0.5 font-semibold ${
                  isSpot ? "bg-sky-500/25 text-sky-200" : "text-[#c5cad3]"
                }`}
              >
                <span className="truncate">
                  {isCallWall && <span className="mr-0.5 text-emerald-400">↑</span>}
                  {isPutWall && <span className="mr-0.5 text-rose-400">↓</span>}
                  {isFlip && <span className="mr-0.5 text-orange-400">◆</span>}
                  {row.strike.toFixed(0)}
                </span>
              </div>
              {row.intensities.map((intensity, i) => {
                const v = row.values[i] ?? 0;
                const strong = Math.abs(intensity) >= 0.12;
                return (
                  <div
                    key={`${row.strike}-${i}`}
                    className={`flex min-h-[22px] items-center justify-center border-b border-white/5 px-0.5 mono text-[8px] font-semibold tabular-nums ${
                      isSpot ? "outline outline-1 outline-sky-400/50" : ""
                    } ${isMoveHigh || isMoveLow ? "outline outline-1 outline-violet-400/40" : ""} ${
                      strong ? "text-white" : "text-white/35"
                    }`}
                    style={{ background: gexCellColor(intensity) }}
                    title={`${row.strike.toFixed(0)} · ${
                      i < cols.length ? cols[i].label : "Σ"
                    }: ${fmtGex(v)}`}
                  >
                    {fmtGex(v)}
                  </div>
                );
              })}
            </div>
          );
        })}

        {/* net row */}
        <div className="border-t border-white/15 px-1 py-1 font-semibold text-[#8b93a7]">Net</div>
        {cols.map((c) => (
          <div
            key={`net-${c.expiry}`}
            className="mono border-t border-white/15 px-0.5 py-1 text-center text-[8px] text-[#c5cad3]"
          >
            {fmtGex(c.net_gex)}
          </div>
        ))}
        <div className="mono border-t border-white/15 px-1 py-1 text-center text-[8px] text-[#c5cad3]">
          {fmtGex(cols.reduce((s, c) => s + c.net_gex, 0))}
        </div>
      </div>
      <div className="flex flex-wrap gap-2 border-t border-white/10 px-2 py-1.5 text-[9px] text-[#8b93a7]">
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-sm bg-emerald-500" /> +γ
        </span>
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-sm bg-rose-500" /> −γ
        </span>
        {spot != null && (
          <span className="text-sky-300">spot {spot.toFixed(2)}</span>
        )}
        {heatmap.move_high != null && heatmap.move_low != null && (
          <span className="text-violet-300">
            move {heatmap.move_low.toFixed(0)}–{heatmap.move_high.toFixed(0)}
          </span>
        )}
      </div>
    </div>
  );
}

function gexCellColor(intensity: number): string {
  // Stronger floors so weak cells aren't near-black empty-looking.
  const a = Math.min(0.92, Math.abs(intensity) * 0.75 + 0.18);
  if (intensity >= 0) {
    return `rgba(22, 163, 74, ${a})`;
  }
  return `rgba(225, 29, 72, ${a})`;
}

function fmtGex(n: number): string {
  const abs = Math.abs(n);
  if (abs < 0.5) return "·";
  if (abs >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (abs >= 10_000) return `${(n / 1_000).toFixed(0)}k`;
  if (abs >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toFixed(0);
}

function VolumeProfilePanel({ vp }: { vp: VolumeProfile }) {
  if (!vp.total_volume) {
    return (
      <div>
        <div className="mb-1 text-[12px] font-bold text-[var(--ink)]">Volume profile</div>
        <div className="text-[11px] text-[var(--text-dim)]">Waiting for prints…</div>
      </div>
    );
  }
  return (
    <div>
      <div className="mb-1">
        <div className="text-[13px] font-bold text-[var(--ink)]">Where volume traded</div>
        <p className="text-[11px] text-[var(--text-dim)]">
          POC = heaviest trade · VAH/VAL = 70% value area
        </p>
      </div>
      <div className="mt-2 grid grid-cols-3 gap-2">
        <LevelChip label="POC" value={vp.poc} accent="#0f172a" />
        <LevelChip label="VAH" value={vp.vah} accent="#0369a1" />
        <LevelChip label="VAL" value={vp.val} accent="#b45309" />
      </div>
      <div className="mt-2 max-h-[120px] space-y-0.5 overflow-auto">
        {vp.levels.slice(0, 16).map((l) => {
          const maxV = Math.max(1, ...vp.levels.slice(0, 16).map((x) => x.volume));
          const w = (l.volume / maxV) * 100;
          return (
            <div
              key={l.price}
              className={`grid grid-cols-[56px_1fr_40px] items-center gap-1.5 text-[10px] ${
                l.is_poc ? "font-bold" : ""
              }`}
            >
              <span className="mono text-right text-[var(--text-muted)]">
                {l.price.toFixed(vp.tick_size < 1 ? 2 : 0)}
              </span>
              <div className="relative h-2.5 overflow-hidden rounded bg-[var(--bg-soft)]">
                <div
                  className="absolute inset-y-0 left-0 rounded"
                  style={{
                    width: `${w}%`,
                    background: l.is_poc
                      ? "#0f172a"
                      : l.in_value_area
                        ? "rgba(3,105,161,0.55)"
                        : "rgba(148,163,184,0.45)",
                  }}
                />
              </div>
              <span className="mono text-right text-[var(--text-dim)]">{fmtInt(l.volume)}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function SessionPanel({ sessions }: { sessions: SessionLevels }) {
  return (
    <div>
      <div className="mb-1">
        <div className="text-[13px] font-bold text-[var(--ink)]">Overnight vs cash session</div>
        <p className="text-[11px] text-[var(--text-dim)]">
          {sessions.in_rth ? "Cash hours live" : "Overnight / pre-open"} — mark these for the open
        </p>
      </div>
      <div className="mt-2 grid grid-cols-2 gap-1.5 sm:grid-cols-3">
        {sessions.levels.map((l) => (
          <div
            key={l.name}
            className="rounded-xl border border-[var(--border)] bg-[var(--bg-soft)] px-2.5 py-2"
          >
            <div className="text-[9px] font-semibold uppercase tracking-wide text-[var(--text-dim)]">
              {l.name}
            </div>
            <div className="mono text-[14px] font-bold text-[var(--ink)]">
              {l.price.toFixed(2)}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function OiPanel({ report, product }: { report: OiMatrixReport; product?: string }) {
  const rows = product
    ? report.rows.filter((r) => r.product === product || r.product === product.replace(/^M/, ""))
    : report.rows;
  const show = rows.length ? rows : report.rows;
  return (
    <div>
      <div className="mb-1">
        <div className="text-[13px] font-bold text-[var(--ink)]">Positioning vs churn</div>
        <p className="text-[11px] text-[var(--text-dim)]">
          Rising volume + sticky OI ≈ new risk · High turnover ≈ day-trade noise
        </p>
      </div>
      <div className="mt-2 space-y-1.5">
        {show.map((r) => (
          <div
            key={r.product}
            className="rounded-xl border border-[var(--border)] px-3 py-2.5"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="text-[12px] font-bold text-[var(--ink)]">
                {r.product}{" "}
                <span className="font-medium text-[var(--text-muted)]">{r.front}</span>
              </span>
              <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase ${
                  r.regime === "positioning"
                    ? "bg-emerald-50 text-emerald-700"
                    : r.regime === "churn"
                      ? "bg-orange-50 text-orange-800"
                      : "bg-[var(--bg-soft)] text-[var(--text-muted)]"
                }`}
              >
                {r.regime}
              </span>
            </div>
            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-[var(--text-dim)]">
              <span>Vol {r.session_volume != null ? fmtInt(r.session_volume) : "—"}</span>
              <span>OI {r.open_interest != null ? fmtInt(r.open_interest) : "—"}</span>
              <span>
                Turnover {r.turnover != null ? `${(r.turnover * 100).toFixed(0)}%` : "—"}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function LevelChip({
  label,
  value,
  accent,
}: {
  label: string;
  value: number;
  accent: string;
}) {
  return (
    <div className="rounded-lg border border-[var(--border)] px-2 py-1.5">
      <div className="text-[9px] font-semibold uppercase tracking-wide" style={{ color: accent }}>
        {label}
      </div>
      <div className="mono text-[14px] font-bold text-[var(--ink)]">{value.toFixed(2)}</div>
    </div>
  );
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString(undefined, {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      fractionalSecondDigits: 3,
    } as Intl.DateTimeFormatOptions);
  } catch {
    return iso.slice(11, 23);
  }
}

function fmtInt(n: number): string {
  return new Intl.NumberFormat("en-US").format(n);
}

function fmtSigned(n: number): string {
  const abs = fmtInt(Math.abs(n));
  if (n > 0) return `+${abs}`;
  if (n < 0) return `−${abs}`;
  return abs;
}

