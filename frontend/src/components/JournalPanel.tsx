"use client";

import { useCallback, useEffect, useState } from "react";
import {
  addJournalTrade,
  fetchJournalAnalytics,
  fetchTradeChart,
  importJournalCsv,
  patchJournalTrade,
} from "@/lib/api";
import type {
  HeatCell,
  JournalAnalytics,
  JournalFilters,
  TradeChart,
  TradeEntry,
} from "@/lib/types";
import { TradeReviewChart } from "./TradeReviewChart";

const SAMPLE_CSV = `symbol,side,qty,entry_price,exit_price,entry_at,exit_at,commissions,setup,account,notes,dte,strategy
SPY,long,3,1.50,2.40,2026-11-17T14:40:00Z,2026-11-17T15:20:00Z,1.5,0DTE Gamma Squeeze,CSV Import,Imported open drive,0,single_leg
QQQ,long,2,2.10,1.20,2026-11-17T16:30:00Z,2026-11-17T17:10:00Z,1.2,Opening Range Breakout,CSV Import,Midday fade,0,single_leg`;

const emptyFilters: JournalFilters = {};

export function JournalPanel() {
  const [filters, setFilters] = useState<JournalFilters>(emptyFilters);
  const [analytics, setAnalytics] = useState<JournalAnalytics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [csv, setCsv] = useState(SAMPLE_CSV);
  const [selected, setSelected] = useState<TradeEntry | null>(null);
  const [chart, setChart] = useState<TradeChart | null>(null);
  const [notes, setNotes] = useState("");
  const [emotion, setEmotion] = useState("");
  const [lesson, setLesson] = useState("");
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async (pickFirst = false) => {
    setLoading(true);
    try {
      const a = await fetchJournalAnalytics(filters);
      setAnalytics(a);
      setError(null);
      setSelected((prev) => {
        if (pickFirst && a.recent[0]) {
          setNotes(a.recent[0].notes);
          setEmotion(a.recent[0].emotion);
          setLesson(a.recent[0].lesson ?? "");
          return a.recent[0];
        }
        if (prev) {
          const still = a.recent.find((t) => t.id === prev.id);
          return still ?? prev;
        }
        return prev;
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load journal");
    } finally {
      setLoading(false);
    }
  }, [filters]);

  useEffect(() => {
    void reload(true);
  }, [reload]);

  useEffect(() => {
    if (!selected) {
      setChart(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const c = await fetchTradeChart(selected.id);
        if (!cancelled) setChart(c);
      } catch {
        if (!cancelled) setChart(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selected?.id]);

  const toggle = (key: keyof JournalFilters, value: string) => {
    setFilters((prev) => ({
      ...prev,
      [key]: prev[key] === value ? null : value,
    }));
  };

  const clearFilters = () => setFilters(emptyFilters);

  const onImport = async () => {
    setBusy(true);
    try {
      const res = await importJournalCsv(csv);
      setAnalytics(res.analytics);
      setError(null);
      await reload(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Import failed");
    } finally {
      setBusy(false);
    }
  };

  const onSaveNotes = async () => {
    if (!selected) return;
    setBusy(true);
    try {
      const trade = await patchJournalTrade(selected.id, {
        notes,
        emotion,
        lesson,
      });
      setSelected(trade);
      await reload(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const onQuickAdd = async () => {
    setBusy(true);
    try {
      await addJournalTrade({
        symbol: "SPY",
        side: "long",
        qty: 2,
        entry_price: 1.75,
        exit_price: 2.55,
        setup: "0DTE Gamma Squeeze",
        account: "Demo-ToS",
        notes: "Logged from UW flow",
        emotion: "focused",
        tags: ["0DTE", "sweep"],
        dte_at_entry: 0,
        contract_strategy: "single_leg",
      });
      await reload(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Add failed");
    } finally {
      setBusy(false);
    }
  };

  const s = analytics?.summary;
  const activeCount = Object.values(filters).filter(Boolean).length;

  return (
    <div className="flex h-full flex-col overflow-hidden bg-[#0B0E14] text-[#e8eaed]">
      <header className="shrink-0 border-b border-[#1c2230] bg-[#0B0E14] px-3 py-2.5 sm:px-5">
        <div className="flex flex-wrap items-center gap-2 sm:gap-3">
          <div>
            <div className="text-[15px] font-extrabold tracking-tight sm:text-[17px]">
              trade<span className="text-sky-400">journal</span>
            </div>
            <div className="hidden text-[10px] font-medium uppercase tracking-[0.12em] text-[#8b93a7] sm:block">
              UW flow context · expectancy · playbooks
            </div>
          </div>
          {activeCount > 0 && (
            <button
              type="button"
              onClick={clearFilters}
              className="rounded-full border border-[#2a3344] px-2.5 py-1 text-[11px] text-[#8b93a7] hover:text-white"
            >
              Clear {activeCount} filter{activeCount > 1 ? "s" : ""}
            </button>
          )}
          <button
            type="button"
            onClick={() => void onQuickAdd()}
            disabled={busy}
            className="ml-auto rounded-full bg-sky-500 px-3 py-1.5 text-[12px] font-semibold text-[#0B0E14] disabled:opacity-50"
          >
            + Log sample fill
          </button>
        </div>
      </header>

      {error && (
        <div className="shrink-0 border-b border-rose-900/50 bg-rose-950/40 px-4 py-2 text-sm text-rose-300">
          {error}
        </div>
      )}

      {loading && !analytics ? (
        <div className="flex flex-1 items-center justify-center text-[#8b93a7]">
          Loading journal…
        </div>
      ) : analytics && s ? (
        <div className="mx-auto grid min-h-0 w-full max-w-[1600px] flex-1 grid-cols-1 gap-0 overflow-auto lg:grid-cols-[1fr_380px]">
          <div className="min-h-0 space-y-0 border-r border-[#1c2230]">
            <section className="border-b border-[#1c2230] bg-[#131722] px-4 py-4">
              <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#8b93a7]">
                Performance · filtered KPIs
              </div>
              <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
                <Metric label="Net P&L" value={money(s.total_pnl)} hot={s.total_pnl} />
                <Metric label="Expectancy" value={money(s.expectancy)} hot={s.expectancy} />
                <Metric label="Profit Factor" value={s.profit_factor.toFixed(2)} />
                <Metric label="Win Rate" value={`${(s.win_rate * 100).toFixed(0)}%`} />
                <Metric label="Sharpe≈" value={s.sharpe_approx.toFixed(2)} />
                <Metric label="Sortino≈" value={s.sortino_approx.toFixed(2)} />
              </div>
              <div className="mt-3 text-[11px] text-[#8b93a7]">
                {s.trade_count} trades · {s.wins}W / {s.losses}L · avg win {money(s.avg_win)} · avg
                loss {money(s.avg_loss)}
              </div>
            </section>

            <section className="border-b border-[#1c2230] px-4 py-3">
              <div className="mb-2 text-[12px] font-bold">Multi-account</div>
              <div className="flex flex-wrap gap-2">
                {analytics.accounts.map((a) => (
                  <button
                    key={a.account}
                    type="button"
                    onClick={() => toggle("account", a.account)}
                    className={`rounded-lg border px-3 py-2 text-left transition-colors ${
                      filters.account === a.account
                        ? "border-sky-500 bg-sky-500/10"
                        : "border-[#1c2230] bg-[#131722] hover:border-[#2a3344]"
                    }`}
                  >
                    <div className="text-[12px] font-bold">{a.account}</div>
                    <div
                      className="mono text-[13px]"
                      style={{ color: a.pnl >= 0 ? "#4ade80" : "#fb7185" }}
                    >
                      {money(a.pnl)}
                    </div>
                    <div className="text-[10px] text-[#8b93a7]">
                      {a.trades} · {(a.win_rate * 100).toFixed(0)}% win
                    </div>
                  </button>
                ))}
              </div>
            </section>

            <section className="grid border-b border-[#1c2230] sm:grid-cols-2">
              <Heatmap
                title="Time of day"
                cells={analytics.time_of_day}
                active={filters.tod}
                onSelect={(label) => toggle("tod", label)}
              />
              <Heatmap
                title="Day of week"
                cells={analytics.day_of_week}
                active={filters.dow}
                onSelect={(label) => toggle("dow", label)}
              />
            </section>

            <section className="border-b border-[#1c2230] px-4 py-3">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <div className="text-[12px] font-bold">Setup playbook</div>
                <div className="ml-auto flex flex-wrap gap-1">
                  {(
                    [
                      ["0dte", "0DTE"],
                      ["1_5", "1–5 DTE"],
                      ["swing", "Swing"],
                    ] as const
                  ).map(([id, label]) => (
                    <button
                      key={id}
                      type="button"
                      onClick={() => toggle("dte_bucket", id)}
                      className={`rounded-full px-2.5 py-0.5 text-[10px] font-semibold ${
                        filters.dte_bucket === id
                          ? "bg-sky-500 text-[#0B0E14]"
                          : "border border-[#2a3344] text-[#8b93a7]"
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
              <PlaybookTable
                rows={analytics.by_setup}
                active={filters.setup}
                onSelect={(setup) => toggle("setup", setup)}
              />
              <div className="mt-3 text-[11px] font-semibold text-[#8b93a7]">Strategy</div>
              <div className="mt-1 flex flex-wrap gap-1.5">
                {analytics.by_strategy.map((row) => (
                  <button
                    key={row.setup}
                    type="button"
                    onClick={() => toggle("strategy", row.setup)}
                    className={`rounded-full px-2.5 py-1 text-[11px] font-semibold ${
                      filters.strategy === row.setup
                        ? "bg-sky-500 text-[#0B0E14]"
                        : "border border-[#2a3344] bg-[#131722] text-[#c5cad3]"
                    }`}
                  >
                    {row.setup.replace(/_/g, " ")} · {money(row.total_pnl)}
                  </button>
                ))}
              </div>
            </section>

            <section className="px-4 py-3">
              <div className="mb-2 text-[12px] font-bold">Trade archive</div>
              <div className="space-y-1.5">
                {analytics.recent.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() => {
                      setSelected(t);
                      setNotes(t.notes);
                      setEmotion(t.emotion);
                      setLesson(t.lesson ?? "");
                    }}
                    className={`flex w-full items-start justify-between gap-3 rounded-lg border px-3 py-2 text-left transition-colors ${
                      selected?.id === t.id
                        ? "border-sky-500 bg-sky-500/10"
                        : "border-[#1c2230] bg-[#131722] hover:border-[#2a3344]"
                    }`}
                  >
                    <div className="min-w-0">
                      <div className="text-[13px] font-bold">
                        {t.symbol}{" "}
                        <span className="font-medium text-[#8b93a7]">
                          {t.side} ×{t.qty}
                        </span>
                        {t.dte_at_entry != null && (
                          <span className="ml-2 rounded bg-[#1c2230] px-1.5 py-0.5 text-[10px] font-semibold text-sky-300">
                            {t.dte_at_entry}DTE
                          </span>
                        )}
                        {t.iv_rank_at_entry != null && (
                          <span className="ml-1 rounded bg-[#1c2230] px-1.5 py-0.5 text-[10px] text-[#8b93a7]">
                            IVR {t.iv_rank_at_entry.toFixed(0)}
                          </span>
                        )}
                      </div>
                      <div className="truncate text-[11px] text-[#8b93a7]">
                        {t.setup} · {t.contract_strategy.replace(/_/g, " ")} · {t.account}
                      </div>
                      {t.flow && <FlowBadges flow={t.flow} compact />}
                    </div>
                    <div
                      className="mono shrink-0 text-[13px] font-bold"
                      style={{ color: t.pnl >= 0 ? "#4ade80" : "#fb7185" }}
                    >
                      {money(t.pnl)}
                    </div>
                  </button>
                ))}
              </div>
            </section>
          </div>

          <aside className="flex min-h-0 flex-col bg-[#0B0E14] lg:overflow-auto">
            <section className="border-b border-[#1c2230] px-4 py-3">
              <div className="mb-1 text-[12px] font-bold">Broker CSV import</div>
              <p className="mb-2 text-[10px] leading-relaxed text-[#8b93a7]">
                Auto-enriches each fill with UW flow alignment + dealer GEX at entry.
              </p>
              <textarea
                value={csv}
                onChange={(e) => setCsv(e.target.value)}
                rows={6}
                className="mono w-full rounded-lg border border-[#1c2230] bg-[#131722] p-2 text-[10px] text-[#e8eaed] outline-none focus:border-sky-500"
              />
              <button
                type="button"
                disabled={busy}
                onClick={() => void onImport()}
                className="mt-2 w-full rounded-full border border-[#2a3344] bg-[#131722] px-3 py-1.5 text-[12px] font-semibold disabled:opacity-50"
              >
                Import + enrich fills
              </button>
            </section>

            <section className="flex-1 px-4 py-3">
              <div className="mb-2 text-[12px] font-bold">Chart review · notes</div>
              {selected ? (
                <>
                  {selected.flow && (
                    <div className="mb-3 space-y-1.5">
                      <FlowBadges flow={selected.flow} />
                    </div>
                  )}
                  <TradeReviewChart chart={chart} />
                  <div className="mt-3 grid grid-cols-3 gap-1.5 text-[10px]">
                    <InfoChip label="Strategy" value={selected.contract_strategy.replace(/_/g, " ")} />
                    <InfoChip
                      label="DTE"
                      value={selected.dte_at_entry != null ? String(selected.dte_at_entry) : "—"}
                    />
                    <InfoChip
                      label="IV Rank"
                      value={
                        selected.iv_rank_at_entry != null
                          ? selected.iv_rank_at_entry.toFixed(0)
                          : "—"
                      }
                    />
                  </div>
                  <label className="mt-3 block text-[10px] font-semibold uppercase text-[#8b93a7]">
                    Emotion
                  </label>
                  <input
                    value={emotion}
                    onChange={(e) => setEmotion(e.target.value)}
                    className="mb-2 w-full rounded-lg border border-[#1c2230] bg-[#131722] px-2 py-1.5 text-[12px] outline-none focus:border-sky-500"
                    placeholder="focused / tilted / patient…"
                  />
                  <label className="block text-[10px] font-semibold uppercase text-[#8b93a7]">
                    Notes
                  </label>
                  <textarea
                    value={notes}
                    onChange={(e) => setNotes(e.target.value)}
                    rows={3}
                    className="w-full rounded-lg border border-[#1c2230] bg-[#131722] px-2 py-1.5 text-[12px] outline-none focus:border-sky-500"
                  />
                  <label className="mt-2 block text-[10px] font-semibold uppercase text-[#8b93a7]">
                    Lesson
                  </label>
                  <textarea
                    value={lesson}
                    onChange={(e) => setLesson(e.target.value)}
                    rows={2}
                    className="w-full rounded-lg border border-[#1c2230] bg-[#131722] px-2 py-1.5 text-[12px] outline-none focus:border-sky-500"
                    placeholder="One actionable takeaway…"
                  />
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void onSaveNotes()}
                    className="mt-2 w-full rounded-full bg-sky-500 px-3 py-1.5 text-[12px] font-semibold text-[#0B0E14] disabled:opacity-50"
                  >
                    Save review
                  </button>
                </>
              ) : (
                <p className="text-[12px] text-[#8b93a7]">Select a trade to annotate.</p>
              )}
            </section>
          </aside>
        </div>
      ) : null}
    </div>
  );
}

function FlowBadges({
  flow,
  compact = false,
}: {
  flow: NonNullable<TradeEntry["flow"]>;
  compact?: boolean;
}) {
  const alignColor =
    flow.alignment === "with"
      ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-300"
      : flow.alignment === "against"
        ? "border-rose-500/40 bg-rose-500/10 text-rose-300"
        : "border-[#2a3344] bg-[#1c2230] text-[#c5cad3]";
  const gexColor =
    flow.gex_regime === "positive_gamma"
      ? "border-sky-500/40 bg-sky-500/10 text-sky-300"
      : flow.gex_regime === "negative_gamma"
        ? "border-amber-500/40 bg-amber-500/10 text-amber-300"
        : "border-[#2a3344] bg-[#1c2230] text-[#8b93a7]";

  if (compact) {
    return (
      <div className="mt-1 truncate text-[10px] text-sky-300/90">{flow.badge}</div>
    );
  }

  return (
    <div className="space-y-1.5">
      <div className={`rounded-lg border px-2.5 py-1.5 text-[11px] font-medium ${alignColor}`}>
        {flow.badge}
      </div>
      <div className={`rounded-lg border px-2.5 py-1.5 text-[11px] font-medium ${gexColor}`}>
        {flow.gex_label}
      </div>
    </div>
  );
}

function PlaybookTable({
  rows,
  active,
  onSelect,
}: {
  rows: JournalAnalytics["by_setup"];
  active?: string | null;
  onSelect: (setup: string) => void;
}) {
  return (
    <div className="overflow-auto">
      <table className="w-full min-w-[480px] text-left text-[12px]">
        <thead className="text-[10px] uppercase tracking-wide text-[#8b93a7]">
          <tr>
            <th className="pb-1 font-semibold">Setup</th>
            <th className="pb-1 font-semibold">n</th>
            <th className="pb-1 font-semibold">Win%</th>
            <th className="pb-1 font-semibold">P&L</th>
            <th className="pb-1 font-semibold">Avg R</th>
            <th className="pb-1 font-semibold">E</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              key={row.setup}
              className={`cursor-pointer border-t border-[#1c2230] ${
                active === row.setup ? "bg-sky-500/10" : "hover:bg-[#131722]"
              }`}
              onClick={() => onSelect(row.setup)}
            >
              <td className="py-1.5 font-medium">{row.setup}</td>
              <td className="mono py-1.5">{row.trades}</td>
              <td className="mono py-1.5">{(row.win_rate * 100).toFixed(0)}%</td>
              <td
                className="mono py-1.5 font-semibold"
                style={{ color: row.total_pnl >= 0 ? "#4ade80" : "#fb7185" }}
              >
                {money(row.total_pnl)}
              </td>
              <td className="mono py-1.5">{row.avg_r.toFixed(2)}</td>
              <td className="mono py-1.5">{money(row.expectancy)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Metric({
  label,
  value,
  hot,
}: {
  label: string;
  value: string;
  hot?: number;
}) {
  const color = hot === undefined ? "#e8eaed" : hot >= 0 ? "#4ade80" : "#fb7185";
  return (
    <div className="rounded-lg border border-[#1c2230] bg-[#0B0E14] px-2.5 py-2">
      <div className="text-[9px] uppercase tracking-wide text-[#8b93a7]">{label}</div>
      <div className="mono text-[16px] font-bold" style={{ color }}>
        {value}
      </div>
    </div>
  );
}

function Heatmap({
  title,
  cells,
  active,
  onSelect,
}: {
  title: string;
  cells: HeatCell[];
  active?: string | null;
  onSelect: (label: string) => void;
}) {
  const maxAbs = Math.max(1, ...cells.map((c) => Math.abs(c.pnl)));
  return (
    <div className="border-b border-[#1c2230] px-4 py-3 sm:border-b-0 sm:border-r sm:border-[#1c2230]">
      <div className="mb-2 text-[12px] font-bold">{title}</div>
      <div className="grid gap-1">
        {cells.map((c) => {
          const intensity = Math.abs(c.pnl) / maxAbs;
          const bg =
            c.trades === 0
              ? "transparent"
              : c.pnl >= 0
                ? `rgba(34,197,94,${0.08 + intensity * 0.35})`
                : `rgba(239,68,68,${0.08 + intensity * 0.35})`;
          return (
            <button
              key={c.label}
              type="button"
              onClick={() => onSelect(c.label)}
              className={`flex items-center justify-between rounded-md border px-2.5 py-1.5 text-left ${
                active === c.label ? "border-sky-500" : "border-[#1c2230]"
              }`}
              style={{ background: bg }}
            >
              <div>
                <div className="text-[11px] font-semibold">{c.label}</div>
                <div className="text-[10px] text-[#8b93a7]">
                  {c.trades} · {(c.win_rate * 100).toFixed(0)}% win
                </div>
              </div>
              <div
                className="mono text-[12px] font-bold"
                style={{ color: c.pnl >= 0 ? "#4ade80" : "#fb7185" }}
              >
                {money(c.pnl)}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function InfoChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-[#1c2230] bg-[#131722] px-2 py-1.5">
      <div className="text-[9px] uppercase text-[#8b93a7]">{label}</div>
      <div className="text-[11px] font-semibold capitalize">{value}</div>
    </div>
  );
}

function money(n: number): string {
  const abs = Math.abs(n).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  });
  if (n > 0) return `+${abs}`;
  if (n < 0) return `−$${abs.replace(/[^0-9.,]/g, "")}`;
  return abs;
}
