"use client";

import { useCallback, useEffect, useState } from "react";
import { fetchSweepClusters } from "@/lib/api";
import type { SweepCluster, SweepClusterReport } from "@/lib/types";

const POLL_MS = 12_000;

export function SweepClustersPanel() {
  const [report, setReport] = useState<SweepClusterReport | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (silent = false) => {
    if (!silent) setLoading(true);
    try {
      const r = await fetchSweepClusters(200);
      setReport(r);
      setSelected((prev) => {
        if (prev && r.clusters.some((c) => c.key === prev)) return prev;
        return r.clusters[0]?.key ?? null;
      });
      setError(null);
    } catch (e) {
      if (!silent) setError(e instanceof Error ? e.message : "Sweep load failed");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  useEffect(() => {
    const id = setInterval(() => void load(true), POLL_MS);
    return () => clearInterval(id);
  }, [load]);

  const active: SweepCluster | null =
    report?.clusters.find((c) => c.key === selected) ?? report?.clusters[0] ?? null;

  if (loading && !report) {
    return (
      <div className="flex flex-1 items-center justify-center text-[var(--text-muted)]">
        Scanning live flow for 0DTE sweep clusters…
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
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="shrink-0 border-b border-[var(--border)] bg-[var(--promo)] px-4 py-4 text-white sm:px-6">
        <div className="mx-auto max-w-7xl">
          <div className="text-[11px] font-semibold uppercase tracking-[0.14em] text-white/45">
            0DTE sweep clusters · {report.session_date}
          </div>
          <h1 className="mt-1 text-[22px] font-extrabold tracking-tight sm:text-[26px]">
            Same Live Flow — filtered for same-day storms
          </h1>
          <p className="mt-2 max-w-3xl text-[13px] text-white/70">{report.narrative}</p>
          <div className="mt-3 flex flex-wrap gap-3 text-[12px]">
            <Pill label="Scanned" value={String(report.total_alerts_scanned)} />
            <Pill label="0DTE" value={String(report.zero_dte_alerts)} />
            <Pill label="Sweeps" value={String(report.sweep_alerts)} />
            <Pill label="Clusters" value={String(report.clusters.length)} />
          </div>
        </div>
      </div>

      <div className="mx-auto grid min-h-0 w-full max-w-7xl flex-1 grid-cols-1 gap-0 lg:grid-cols-[340px_minmax(0,1fr)]">
        <aside className="min-h-0 overflow-auto border-r border-[var(--border)] bg-white">
          <div className="sticky top-0 border-b border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2 text-[11px] font-bold uppercase tracking-wide text-[var(--text-dim)]">
            Clusters
          </div>
          {report.clusters.length === 0 && (
            <div className="p-4 text-[13px] text-[var(--text-muted)]">
              No 0DTE clusters in this window.
            </div>
          )}
          {report.clusters.map((c) => (
            <button
              key={c.key}
              type="button"
              onClick={() => setSelected(c.key)}
              className={`flex w-full flex-col border-b border-[var(--border)] px-3 py-3 text-left ${
                active?.key === c.key ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--bg-soft)]"
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-bold text-[var(--ink)]">{c.ticker}</span>
                <span
                  className={`rounded-full px-2 py-0.5 text-[10px] font-bold ${
                    c.cluster_score >= 75
                      ? "bg-rose-50 text-rose-700"
                      : c.cluster_score >= 50
                        ? "bg-orange-50 text-orange-700"
                        : "bg-zinc-100 text-zinc-600"
                  }`}
                >
                  {c.label}
                </span>
              </div>
              <div className="mt-1 mono text-[18px] font-extrabold text-[var(--ink)]">
                {c.cluster_score}
                <span className="ml-2 text-[11px] font-medium text-[var(--text-dim)]">
                  ${fmtPrem(c.total_premium)}
                </span>
              </div>
              <div className="mt-1 text-[11px] text-[var(--text-muted)]">
                {c.alert_count} alerts · {c.sweep_count} sweeps · {c.dominant_side} ·{" "}
                {c.bucket_start.slice(11)}
              </div>
            </button>
          ))}
        </aside>

        <section className="min-h-0 overflow-auto bg-[var(--bg-soft)] p-4">
          {active ? (
            <div className="space-y-4">
              <div className="rounded-2xl border border-[var(--border)] bg-white p-4">
                <div className="flex flex-wrap items-end justify-between gap-2">
                  <div>
                    <div className="text-[11px] font-semibold uppercase tracking-wide text-[var(--text-dim)]">
                      Cluster detail
                    </div>
                    <div className="text-[22px] font-extrabold text-[var(--ink)]">
                      {active.ticker}{" "}
                      <span className="text-[14px] font-semibold text-[var(--text-muted)]">
                        {active.bucket_start}
                      </span>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="mono text-[24px] font-extrabold text-[var(--ink)]">
                      ${fmtPrem(active.total_premium)}
                    </div>
                    <div className="text-[11px] text-[var(--text-muted)]">
                      call ${fmtPrem(active.call_premium)} · put ${fmtPrem(active.put_premium)}
                    </div>
                  </div>
                </div>
                <div className="mt-3 flex h-2 overflow-hidden rounded-full bg-[var(--bg-soft)]">
                  <div
                    className="h-full bg-emerald-500"
                    style={{
                      width: `${share(active.call_premium, active.put_premium)}%`,
                    }}
                  />
                  <div className="h-full bg-rose-500" style={{ flex: 1 }} />
                </div>
              </div>

              <div className="overflow-hidden rounded-2xl border border-[var(--border)] bg-white">
                <table className="w-full min-w-[640px] text-left text-[12px]">
                  <thead className="border-b border-[var(--border)] bg-[var(--bg-soft)] text-[10px] uppercase tracking-wide text-[var(--text-dim)]">
                    <tr>
                      <th className="px-3 py-2">Time</th>
                      <th className="px-2 py-2">Type</th>
                      <th className="px-2 py-2">Strike</th>
                      <th className="px-2 py-2">Prem</th>
                      <th className="px-2 py-2">Side</th>
                      <th className="px-3 py-2">Flags</th>
                    </tr>
                  </thead>
                  <tbody>
                    {active.alerts.map((a) => (
                      <tr key={a.id} className="border-b border-[var(--border)]">
                        <td className="mono px-3 py-2 text-[var(--text-dim)]">
                          {a.created_at.slice(11, 19)}
                        </td>
                        <td
                          className={`px-2 py-2 font-semibold ${
                            a.option_type.toLowerCase() === "put"
                              ? "text-[var(--put)]"
                              : "text-[var(--call)]"
                          }`}
                        >
                          {a.option_type}
                        </td>
                        <td className="mono px-2 py-2">{a.strike}</td>
                        <td className="mono px-2 py-2 font-semibold">${fmtPrem(a.premium)}</td>
                        <td className="px-2 py-2 uppercase text-[11px] text-[var(--text-muted)]">
                          {a.side_bias}
                        </td>
                        <td className="px-3 py-2">
                          {a.has_sweep && (
                            <span className="rounded bg-orange-100 px-1.5 py-0.5 text-[9px] font-bold uppercase text-orange-800">
                              Sweep
                            </span>
                          )}
                          <span className="ml-1 text-[10px] text-[var(--text-dim)]">
                            {a.alert_rule}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="rounded-2xl border border-[var(--border)] bg-white p-4">
                <div className="mb-2 text-[13px] font-bold text-[var(--ink)]">
                  Recent 0DTE feed
                </div>
                <div className="max-h-48 space-y-1 overflow-auto text-[12px]">
                  {report.feed.slice(0, 40).map((a) => (
                    <div
                      key={`${a.id}-${a.created_at}`}
                      className="flex flex-wrap items-center gap-2 border-b border-[var(--border)]/60 py-1.5"
                    >
                      <span className="mono text-[var(--text-dim)]">
                        {a.created_at.slice(11, 19)}
                      </span>
                      <span className="font-bold">{a.ticker}</span>
                      <span
                        className={
                          a.option_type.toLowerCase() === "put"
                            ? "text-[var(--put)]"
                            : "text-[var(--call)]"
                        }
                      >
                        {a.option_type} {a.strike}
                      </span>
                      <span className="mono">${fmtPrem(a.premium)}</span>
                      {a.has_sweep && (
                        <span className="text-[10px] font-bold uppercase text-orange-700">
                          sweep
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <div className="text-[13px] text-[var(--text-muted)]">Select a cluster</div>
          )}
          <p className="mt-4 text-[11px] text-[var(--text-dim)]">{report.methodology}</p>
        </section>
      </div>
    </div>
  );
}

function Pill({ label, value }: { label: string; value: string }) {
  return (
    <span className="rounded-full bg-white/10 px-2.5 py-1">
      <span className="text-white/50">{label} </span>
      <span className="font-bold">{value}</span>
    </span>
  );
}

function fmtPrem(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(0);
}

function share(callPrem: number, putPrem: number): number {
  const t = callPrem + putPrem;
  if (t <= 0) return 50;
  return (callPrem / t) * 100;
}
