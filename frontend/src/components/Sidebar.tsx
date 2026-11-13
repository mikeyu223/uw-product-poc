const NAV = [
  { id: "dashboard", label: "Home", glyph: "⌂", active: true },
  { id: "positions", label: "Book", glyph: "▦", active: false },
  { id: "flow", label: "Flow", glyph: "↗", active: false },
  { id: "gex", label: "GEX", glyph: "Σ", active: false },
  { id: "alerts", label: "Alerts", glyph: "◉", active: false },
];

export function Sidebar({ backendOk }: { backendOk: boolean | null }) {
  return (
    <>
      {/* Desktop left rail */}
      <aside className="hidden w-[64px] shrink-0 flex-col items-center border-r border-[var(--border)] bg-white py-3 md:flex">
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

      {/* Mobile bottom nav */}
      <nav className="fixed inset-x-0 bottom-0 z-40 flex items-center justify-around border-t border-[var(--border)] bg-white/95 px-2 pb-[max(0.5rem,env(safe-area-inset-bottom))] pt-2 backdrop-blur md:hidden">
        {NAV.map((item) => (
          <div
            key={item.id}
            className={`flex min-w-[3.25rem] flex-col items-center gap-0.5 rounded-lg px-2 py-1 ${
              item.active ? "text-[var(--ink)]" : "text-[var(--text-dim)]"
            }`}
          >
            <span className="text-[16px]" aria-hidden>
              {item.glyph}
            </span>
            <span className="text-[10px] font-medium">{item.label}</span>
          </div>
        ))}
        <div
          className={`h-2 w-2 rounded-full ${
            backendOk === null
              ? "bg-zinc-300"
              : backendOk
                ? "bg-emerald-500"
                : "bg-rose-500"
          }`}
          title="Backend"
        />
      </nav>
    </>
  );
}
