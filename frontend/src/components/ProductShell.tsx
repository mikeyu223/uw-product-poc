"use client";

import { useState } from "react";
import { CvdDashboard } from "./CvdDashboard";
import { JournalPanel } from "./JournalPanel";

type Mode = "tape" | "journal";

const MODES: Array<{ id: Mode; label: string; blurb: string }> = [
  {
    id: "tape",
    label: "Futures Order Flow",
    blurb: "CVD · GEX flip · block tape · POC/VA · Globex/RTH · OI",
  },
  {
    id: "journal",
    label: "Trading Journal",
    blurb: "CSV import · expectancy · TOD heatmap · playbooks",
  },
];

export function ProductShell() {
  const [mode, setMode] = useState<Mode>("tape");
  const dark = mode === "journal";

  return (
    <div
      className={`flex h-[100dvh] flex-col overflow-hidden ${
        dark ? "bg-[#0B0E14]" : "bg-[var(--bg-soft)]"
      }`}
    >
      <nav
        className={`shrink-0 border-b px-3 py-2 sm:px-5 ${
          dark
            ? "border-[#1c2230] bg-[#0B0E14]"
            : "border-[var(--border)] bg-white"
        }`}
      >
        <div className="mx-auto flex max-w-[1600px] flex-wrap items-center gap-2">
          <div
            className={`mr-2 shrink-0 text-[14px] font-extrabold tracking-tight ${
              dark ? "text-white" : "text-[var(--ink)]"
            }`}
          >
            uw<span className={dark ? "text-sky-400" : "text-[var(--accent)]"}>poc</span>
          </div>
          {MODES.map((m) => (
            <button
              key={m.id}
              type="button"
              title={m.blurb}
              onClick={() => setMode(m.id)}
              className={`rounded-full px-3 py-1.5 text-[12px] font-semibold transition-colors ${
                mode === m.id
                  ? dark
                    ? "bg-sky-500 text-[#0B0E14]"
                    : "bg-[var(--ink)] text-white"
                  : dark
                    ? "border border-[#2a3344] text-[#8b93a7] hover:text-white"
                    : "border border-[var(--border)] text-[var(--text-muted)] hover:text-[var(--ink)]"
              }`}
            >
              {m.label}
            </button>
          ))}
          <span
            className={`ml-auto hidden text-[11px] lg:inline ${
              dark ? "text-[#8b93a7]" : "text-[var(--text-dim)]"
            }`}
          >
            {MODES.find((m) => m.id === mode)?.blurb}
          </span>
        </div>
      </nav>

      <div className="min-h-0 flex-1 overflow-hidden">
        {mode === "tape" && <CvdDashboard embedded />}
        {mode === "journal" && <JournalPanel />}
      </div>
    </div>
  );
}
