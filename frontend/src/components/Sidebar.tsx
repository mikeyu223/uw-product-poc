const NAV = [
  { id: "dashboard", label: "Home", glyph: "⌂", active: true },
  { id: "positions", label: "Book", glyph: "▦", active: false },
  { id: "flow", label: "Flow", glyph: "↗", active: false },
  { id: "gex", label: "GEX", glyph: "Σ", active: false },
  { id: "alerts", label: "Alerts", glyph: "◉", active: false },
];

export function Sidebar({ backendOk }: { backendOk: boolean | null }) {
  return (
    <aside className="flex w-[64px] shrink-0 flex-col items-center border-r border-[var(--border)] bg-white py-3">
      <div
        className="mb-4 flex h-9 w-9 items-center justify-center rounded-full bg-[var(--ink)] text-[11px] font-bold tracking-tight text-white"
        title="Risk Watcher"
      >
        rw
      </div>

      <nav className="flex flex-1 flex-col items-center gap-1.5">
        {NAV.map((item) => (
          <div
            key={item.id}
            title={item.label}
            className={`flex h-10 w-10 items-center justify-center rounded-full text-[15px] transition-colors ${
              item.active
                ? "bg-[var(--bg-soft)] text-[var(--ink)] ring-1 ring-[var(--border)]"
                : "text-[var(--text-dim)] hover:bg-[var(--bg-soft)] hover:text-[var(--text)]"
            }`}
          >
            <span aria-hidden>{item.glyph}</span>
            <span className="sr-only">{item.label}</span>
          </div>
        ))}
      </nav>

      <div
        className={`mb-1 h-2.5 w-2.5 rounded-full ${
          backendOk === null
            ? "bg-zinc-300"
            : backendOk
              ? "bg-emerald-500"
              : "bg-rose-500"
        }`}
        title={backendOk ? "Backend online" : backendOk === null ? "…" : "Backend offline"}
      />
    </aside>
  );
}
