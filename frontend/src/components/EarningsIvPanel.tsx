"use client";

import { useCallback, useEffect, useState } from "react";
import { fetchEarningsIv } from "@/lib/api";
import type { EarningsName, EarningsIvReport } from "@/lib/types";

export function EarningsIvPanel() {
  const [report, setReport] = useState<EarningsIvReport | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const r = await fetchEarningsIv();
      setReport(r);
      setSelected((prev) => prev ?? r.names[0]?.ticker ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "IV crush load failed");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const active: EarningsName | null =
    report?.names.find((n) => n.ticker === selected) ?? report?.names[0] ?? null;

  if (loading && !report) {
    return (
      <div className="flex flex-1 items-center justify-center text-[var(--text-muted)]">
        Loading earnings IV surfaces…
      </div>
    );
  }

  if (error && !report) {
    return (
      <div className="m-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
        {error}
      </div>
    );
  }

  if (!report) return null;

  return (
    <div className="mx-auto flex min-h-0 w-full max-w-7xl flex-1 flex-col gap-4 overflow-auto p-4 sm:p-6">
      <div className="rounded-2xl border border-[var(--border)] bg-[var(--promo)] px-5 py-5 text-white">
        <div className="text-[11px] font-semibold uppercase tracking-[0.14em] text-white/45">
          Earnings IV crush
        </div>
        <h1 className="mt-2 text-[24px] font-extrabold tracking-tight sm:text-[28px]">
          Front-end vol vs the rest of the curve
        </h1>
        <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-white/70">
          {report.narrative}
        </p>
        <button
          type="button"
          onClick={() => void load()}
          className="mt-4 rounded-full border border-white/20 bg-white/10 px-3 py-1.5 text-[12px] font-semibold"
        >
          Refresh watchlist
        </button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {report.names.map((n) => (
          <button
            key={n.ticker}
            type="button"
            onClick={() => setSelected(n.ticker)}
            className={`rounded-xl border p-3 text-left transition-colors ${
              active?.ticker === n.ticker
                ? "border-[var(--ink)] bg-white ring-1 ring-[var(--ink)]"
                : "border-[var(--border)] bg-white hover:border-[var(--border-strong)]"
            }`}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-bold text-[var(--ink)]">{n.ticker}</span>
              <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-bold ${
                  n.crush_pressure >= 75
                    ? "bg-rose-50 text-rose-700"
                    : n.crush_pressure >= 50
                      ? "bg-orange-50 text-orange-700"
                      : "bg-sky-50 text-sky-700"
                }`}
              >
                {n.crush_label}
              </span>
            </div>
            <div
              className="mono mt-2 text-[22px] font-extrabold"
              style={{ color: pressureColor(n.crush_pressure) }}
            >
              {n.crush_pressure}
            </div>
            <div className="mt-1 text-[11px] text-[var(--text-muted)]">
              {n.days_to_earnings != null
                ? `${n.days_to_earnings}d to earnings`
                : "No date"}
              {n.crush_ratio != null ? ` · front/30d ${n.crush_ratio.toFixed(2)}×` : ""}
            </div>
          </button>
        ))}
      </div>

      {active && (
        <div className="grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
          <div className="rounded-2xl border border-[var(--border)] bg-white p-4 sm:p-5">
            <div className="mb-1 text-[15px] font-bold text-[var(--ink)]">
              {active.ticker} term structure
            </div>
            <p className="mb-4 text-[12px] text-[var(--text-muted)]">{active.takeaway}</p>
            <TermBars term={active.term} />
            <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
              <Metric label="IV rank" value={active.iv_rank != null ? `${active.iv_rank.toFixed(0)}` : "—"} />
              <Metric
                label="Front IV"
                value={active.front_iv != null ? pct(active.front_iv) : "—"}
              />
              <Metric label="30d IV" value={active.iv_30d != null ? pct(active.iv_30d) : "—"} />
              <Metric
                label="Impl move"
                value={
                  active.implied_move_pct != null
                    ? `${active.implied_move_pct.toFixed(1)}%`
                    : "—"
                }
              />
            </div>
          </div>

          <div className="rounded-2xl border border-[var(--border)] bg-white p-4 sm:p-5">
            <div className="mb-1 text-[15px] font-bold text-[var(--ink)]">IV rank history</div>
            <p className="mb-3 text-[12px] text-[var(--text-muted)]">
              Last ~60 sessions · earnings{" "}
              {active.earnings_date?.slice(0, 10) ?? "n/a"}
              {active.earnings_confirmed ? " (confirmed)" : ""}
            </p>
            <IvSparkline history={active.iv_history} />
            <div className="mt-4 text-[11px] text-[var(--text-dim)]">
              Spot {active.spot != null ? active.spot.toFixed(2) : "—"} ·{" "}
              {active.sector ?? "—"}
            </div>
          </div>
        </div>
      )}

      <p className="text-[11px] text-[var(--text-dim)]">{report.methodology}</p>
    </div>
  );
}

function TermBars({ term }: { term: EarningsName["term"] }) {
  const max = Math.max(...term.map((t) => t.iv), 0.01);
  return (
    <div className="space-y-2">
      {term.map((t) => (
        <div key={t.days} className="grid grid-cols-[52px_1fr_56px] items-center gap-2 text-[12px]">
          <span className="mono text-[var(--text-muted)]">{t.days}d</span>
          <div className="h-3 overflow-hidden rounded bg-[var(--bg-soft)]">
            <div
              className="h-full rounded bg-[var(--accent)]"
              style={{ width: `${(t.iv / max) * 100}%` }}
            />
          </div>
          <span className="mono text-right font-semibold text-[var(--ink)]">{pct(t.iv)}</span>
        </div>
      ))}
    </div>
  );
}

function IvSparkline({ history }: { history: EarningsName["iv_history"] }) {
  if (history.length < 2) {
    return <div className="text-[12px] text-[var(--text-dim)]">No IV history</div>;
  }
  const w = 420;
  const h = 120;
  const pad = 8;
  const vals = history.map((p) => p.iv_rank ?? p.iv * 100);
  const min = Math.min(...vals);
  const max = Math.max(...vals);
  const span = Math.max(1, max - min);
  const d = history
    .map((p, i) => {
      const v = p.iv_rank ?? p.iv * 100;
      const x = pad + (i / (history.length - 1)) * (w - pad * 2);
      const y = pad + (1 - (v - min) / span) * (h - pad * 2);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" L ");
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="h-[120px] w-full">
      <path d={`M ${d}`} fill="none" stroke="#2563eb" strokeWidth={2} strokeLinejoin="round" />
    </svg>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2">
      <div className="text-[10px] font-semibold uppercase tracking-wide text-[var(--text-dim)]">
        {label}
      </div>
      <div className="mono mt-0.5 text-[16px] font-bold text-[var(--ink)]">{value}</div>
    </div>
  );
}

function pct(iv: number): string {
  return `${(iv * 100).toFixed(1)}%`;
}

function pressureColor(p: number): string {
  if (p >= 75) return "#e11d48";
  if (p >= 50) return "#ea580c";
  if (p >= 25) return "#0284c7";
  return "#16a34a";
}
