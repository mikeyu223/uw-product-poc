"use client";

import { useEffect, useRef } from "react";
import { ColorType, createChart, type IChartApi, type ISeriesApi, type Time } from "lightweight-charts";
import type { TradeChart } from "@/lib/types";

export function TradeReviewChart({ chart }: { chart: TradeChart | null }) {
  const host = useRef<HTMLDivElement>(null);
  const api = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);

  useEffect(() => {
    if (!host.current) return;
    const c = createChart(host.current, {
      height: 220,
      layout: {
        background: { type: ColorType.Solid, color: "#131722" },
        textColor: "#8b93a7",
      },
      grid: {
        vertLines: { color: "#1c2230" },
        horzLines: { color: "#1c2230" },
      },
      rightPriceScale: { borderColor: "#1c2230" },
      timeScale: { borderColor: "#1c2230", timeVisible: true, secondsVisible: false },
      crosshair: { vertLine: { color: "#334155" }, horzLine: { color: "#334155" } },
    });
    api.current = c;
    const series = c.addCandlestickSeries({
      upColor: "#22c55e",
      downColor: "#ef4444",
      borderVisible: false,
      wickUpColor: "#22c55e",
      wickDownColor: "#ef4444",
    });
    seriesRef.current = series;

    const ro = new ResizeObserver(() => {
      if (host.current) c.applyOptions({ width: host.current.clientWidth });
    });
    ro.observe(host.current);
    return () => {
      ro.disconnect();
      c.remove();
      api.current = null;
      seriesRef.current = null;
    };
  }, []);

  useEffect(() => {
    const series = seriesRef.current;
    const c = api.current;
    if (!series || !c || !chart?.candles?.length) return;

    const candles = [...chart.candles].sort((a, b) => a.time - b.time);
    const candleTimes = candles.map((b) => b.time);

    series.setData(
      candles.map((b) => ({
        time: b.time as Time,
        open: b.open,
        high: b.high,
        low: b.low,
        close: b.close,
      })),
    );

    // lightweight-charts requires markers asc by time and aligned to bar times.
    const markers = [...chart.markers]
      .map((m) => ({
        ...m,
        time: nearestBarTime(m.time, candleTimes),
      }))
      .sort((a, b) => a.time - b.time || a.kind.localeCompare(b.kind));

    series.setMarkers(
      markers.map((m) => ({
        time: m.time as Time,
        position: m.position as "aboveBar" | "belowBar" | "inBar",
        color: m.color,
        shape: m.shape as "circle" | "square" | "arrowUp" | "arrowDown",
        text: m.text,
      })),
    );
    c.timeScale().fitContent();
  }, [chart]);

  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-[10px] text-[#8b93a7]">
        <span>{chart ? `${chart.symbol} · ±30m around fill` : "Review chart"}</span>
        {chart && (
          <span className="uppercase tracking-wide">{chart.source.split("+")[0]}</span>
        )}
      </div>
      <div
        ref={host}
        className="w-full overflow-hidden rounded-lg bg-[#131722]"
        style={{ minHeight: 220 }}
      />
      <div className="mt-1.5 flex flex-wrap gap-2 text-[10px] text-[#8b93a7]">
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-full bg-emerald-500" /> Entry
        </span>
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-full bg-rose-500" /> Exit
        </span>
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-full bg-sky-400" /> Call sweep
        </span>
        <span className="inline-flex items-center gap-1">
          <span className="h-2 w-2 rounded-full bg-pink-400" /> Put sweep
        </span>
      </div>
    </div>
  );
}

/** Snap marker timestamps onto existing candle times (required by lightweight-charts). */
function nearestBarTime(t: number, bars: number[]): number {
  if (!bars.length) return t;
  let best = bars[0];
  let bestDist = Math.abs(bars[0] - t);
  for (let i = 1; i < bars.length; i++) {
    const d = Math.abs(bars[i] - t);
    if (d < bestDist) {
      best = bars[i];
      bestDist = d;
    }
  }
  return best;
}
