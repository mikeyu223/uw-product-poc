# Risk Watcher PoC — How Everything Works

This document explains the **structure**, **data flow**, and **responsibilities** of the Autonomous Risk Watcher proof-of-concept.

**Audience:** engineers and product folks building on this PoC.  
**Product audience:** options traders who need position-aware microstructure risk (GEX + flow), not just raw alerts.

---

## 1. What the system does

Risk Watcher maps an **options portfolio** to **Unusual Whales (UW) market microstructure**, then produces:

1. A **0–100 risk score** per position  
2. A **risk bucket** (Low / Moderate / Elevated / High)  
3. **Clear drivers** — plain-language explanations of *why* the score is high or low  

```
┌─────────────┐     ┌──────────────────────┐     ┌─────────────────┐
│ Sample book │────▶│ Rust risk engine     │────▶│ Next.js dashboard│
│ (positions) │     │ + UW API client      │     │ (dark UI)        │
└─────────────┘     └──────────┬───────────┘     └─────────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │ Unusual Whales API   │
                    │ GEX · flow · spot ·  │
                    │ greek exposure ·     │
                    │ earnings             │
                    └──────────────────────┘
```

### What is real vs sample today

| Piece | Status |
|--------|--------|
| Unusual Whales GEX, flow, spot, greek exposure, earnings | **Live API** |
| Risk scores + drivers | **Computed live** from UW data |
| The 5 portfolio positions (strikes / qty) | **Sample book** (not brokerage sync) |
| Kafka / streaming ingest | **Stub** (simulated loop only) |

---

## 2. Repository layout

```
risk-watcher-poc/
├── .env / .env.example          # UW API key + bind addr (never commit .env)
├── README.md                    # Quick start
├── docs/
│   └── ARCHITECTURE.md          # This file
├── backend/                     # Rust engine (Tokio + Axum + reqwest)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # CLI --probe + HTTP server entry
│       ├── lib.rs               # Module exports
│       ├── config.rs            # Env / dotenv loading
│       ├── api/                 # Axum HTTP routes
│       ├── uw/                  # Unusual Whales REST client
│       ├── risk/                # Scoring + sample portfolio
│       └── ingest/              # Simulated Kafka-like consumer (stub)
└── frontend/                    # Next.js + Tailwind dashboard
    └── src/
        ├── app/                 # Next.js App Router pages
        ├── components/          # Dashboard UI
        └── lib/                 # API client + types + risk helpers
```

---

## 3. Runtime topology

```
Browser  ──HTTP──▶  Next.js :3000  ──HTTP──▶  Axum :8080  ──HTTPS──▶  api.unusualwhales.com
                         │                         │
                         │                         ├── GET /health
                         │                         ├── GET /api/v1/portfolio
                         │                         ├── GET /api/v1/positions
                         │                         ├── GET /api/v1/methodology
                         │                         └── GET /api/v1/uw/probe/:ticker
                         │
                         └── NEXT_PUBLIC_API_URL=http://127.0.0.1:8080
```

**Start locally**

```bash
# Terminal 1 — backend
cd backend && cargo run

# Terminal 2 — frontend
cd frontend && NEXT_PUBLIC_API_URL=http://127.0.0.1:8080 npm run dev -- -p 3000

# Optional — CLI probe without server
cd backend && cargo run -- --probe SPY
```

---

## 4. Backend modules (Rust)

### 4.1 `config` — configuration

Loads `.env` from the repo root (or CWD) and reads:

| Variable | Purpose |
|----------|---------|
| `UNUSUAL_WHALES_API_KEY` | Bearer token (UUID) |
| `UW_BASE_URL` | Default `https://api.unusualwhales.com` |
| `BIND_ADDR` | Default `127.0.0.1:8080` |
| `RUST_LOG` | Tracing filter |

### 4.2 `uw` — Unusual Whales client

| File | Role |
|------|------|
| `client.rs` | Async `reqwest` client; auth header; endpoint helpers |
| `models.rs` | Serde types for UW JSON (`GexLevels`, `FlowAlert`, `StockState`, …) |
| `error.rs` | Typed errors (`Api`, `Http`, `Deserialize`) |

**Auth:** `Authorization: Bearer <token>`

**Endpoints used for scoring**

| Method | UW path | Used for |
|--------|---------|----------|
| `stock_state` | `/api/stock/{t}/stock-state` | Live spot |
| `gex_levels` | `/api/stock/{t}/gex-levels?source=vol` | Flip, call/put walls, magnet |
| `latest_greek_exposure` | `/api/stock/{t}/greek-exposure` | Net dealer gamma regime |
| `flow_alerts` | `/api/stock/{t}/flow-alerts` | Unusual flow / sweeps |
| `ticker_info` | `/api/stock/{t}/info` | Earnings / catalyst |

`probe(ticker)` is a convenience that hits info + GEX + greeks + a few flow alerts for connectivity checks (CLI and `/api/v1/uw/probe/:ticker`).

### 4.3 `risk` — scoring & explanation engine

```
risk/
├── mod.rs        # Public types: RiskBucket, RiskDriver, PositionRisk, ScoreBreakdown
├── engine.rs     # score_position(position, market_context) → PositionRisk
└── portfolio.rs  # sample_book() + score_book(uw) → Vec<ScoredPosition>
```

**Data flow inside scoring**

```
PositionInput (sample strike/side/qty/greeks)
        +
MarketContext (live spot, GEX, greek exposure, flow, ticker info)
        │
        ▼
score_position()
        │
        ├── gamma regime        (max 26)
        ├── flip proximity      (max 20)
        ├── wall conflict       (max 16)
        ├── adverse flow        (max 20)
        ├── structure / DTE     (max 10)
        └── catalyst / earnings (max 8)
        │
        ▼
PositionRisk { score 0–100, bucket, drivers[], breakdown, methodology }
```

**Why this mix (options traders)**

| Family | Max | Trader meaning |
|--------|-----|----------------|
| Dealer GEX regime | 26 | Neg GEX → hedging can *accelerate* moves |
| Zero-gamma flip | 20 | Near flip → regime can switch dampening ↔ amplifying |
| Wall conflict | 16 | Call/put walls as headwinds or magnets vs *your* strike |
| Adverse flow | 20 | Live sweeps / ask-side premium working against the position |
| Structure | 10 | Short DTE + fragile OTM structures |
| Catalyst | 8 | Earnings / binary event window |

Drivers are sorted by contribution and written in plain language (e.g. “Spot is 0.03% above the gamma flip…”).

**Buckets**

| Score | Bucket |
|-------|--------|
| 0–24 | Low |
| 25–49 | Moderate |
| 50–74 | Elevated |
| 75–100 | High |

### 4.4 `api` — HTTP surface (Axum)

| Route | Behavior |
|-------|----------|
| `GET /health` | Liveness |
| `GET /api/v1/methodology` | Static explanation of the score mix |
| `GET /api/v1/positions` | Score sample book via live UW → JSON positions |
| `GET /api/v1/portfolio` | Aggregate summary (avg score, top drivers, `mode: live_uw`) |
| `GET /api/v1/uw/probe/{ticker}` | Raw UW probe payload for UI sanity checks |

CORS is permissive for local frontend development.

### Caching (important)

Previously each dashboard load called `/portfolio` **and** `/positions`, and each handler fully re-scored the book against Unusual Whales (~25–50 HTTP calls, multi-second waits).

Now:

| Mechanism | Behavior |
|-----------|----------|
| In-memory `BookCache` | Shared scored book, **45s TTL** |
| Singleflight | Concurrent refreshes coalesce into one UW pull |
| Parallel ticker fetch | Per-position UW calls use `tokio::join!` + concurrent positions |
| `GET /api/v1/book` | Preferred single endpoint (positions + summary + cache meta) |
| `?refresh=1` | Force bypass TTL (Refresh button) |
| Stale fallback | If UW fails but cache exists, serve last good book |

Frontend loads `/api/v1/book` once and shows **Cached Ns** vs **Fresh UW**.

### 4.5 `ingest` — streaming stub

Placeholder for a Kafka-like tick / GEX consumer loop. Production intent: push market events onto an async channel and re-score positions without full REST polling. **Not wired into the live path yet.**

---

## 5. Frontend modules (Next.js)

| Path | Role |
|------|------|
| `app/page.tsx` | Renders `<Dashboard />` |
| `app/layout.tsx` | Dark theme fonts (DM Sans + IBM Plex Mono) |
| `components/Dashboard.tsx` | Main UI: methodology, summary, positions table, drivers, probe |
| `components/Sidebar.tsx` | Nav + backend online indicator |
| `lib/api.ts` | `fetch` helpers → Axum API |
| `lib/types.ts` | TypeScript mirrors of backend JSON |
| `lib/risk.ts` | Bucket colors / labels for the UI |

**UI sections**

1. **How the risk score works** — mix cards from `/api/v1/methodology`  
2. **Summary strip** — portfolio avg, elevated count, top driver  
3. **Positions table** — sample legs + live scores; click a row  
4. **Why this score** — GEX snapshot, point breakdown bars, driver details  
5. **Raw UW Probe** — independent live GEX/flow check for any ticker  

Badge **Live UW scoring** appears when `summary.mode === "live_uw"`.  
**Offline / demo** means the portfolio fetch failed or did not return live mode (backend down, network, or stale tab).

---

## 6. End-to-end request path

When you open the dashboard or click **Refresh scores**:

```
1. Browser → GET /health
2. Browser → GET /api/v1/portfolio  ─┐
3. Browser → GET /api/v1/positions  ─┼─▶ Axum
4. Browser → GET /api/v1/methodology┘
5. For each sample position ticker:
      Axum → UW stock-state, gex-levels, greek-exposure, flow-alerts, info
6. Axum runs score_position() per leg
7. JSON returns with score, bucket, drivers, breakdown, gex_snapshot
8. Dashboard renders table + “Why this score” panel
```

---

## 7. Key design decisions

1. **Position-aware, not market-only** — the same SPY GEX tape scores differently for a long put near the put wall vs a long call into the call wall.  
2. **Explainability first** — every score ships with ranked drivers and a numeric breakdown traders can audit.  
3. **Live microstructure, sample book** — proves UW + scoring without needing brokerage OAuth on day one.  
4. **REST polling PoC** — simple and debuggable; Kafka/websocket ingest is the natural next step for latency.  
5. **Trader vocabulary** — labels talk about dealer gamma, flips, walls, sweeps — not generic “risk factor #3”.

---

## 8. What’s next (natural roadmap)

1. **Brokerage / account sync** — replace `sample_book()` with real user positions  
2. **Cache / batch UW fetches** — avoid re-hitting all tickers on every page load  
3. **Wire `ingest`** — push GEX/flow updates; incremental re-score  
4. **Alerts** — notify when a position crosses Elevated/High or a new top driver appears  
5. **Backtests / calibration** — tune the 26/20/16/20/10/8 weights against historical P&amp;L outcomes  

---

## 9. Quick file map (if you’re jumping into code)

| Want to change… | Start here |
|-----------------|------------|
| Score weights / driver text | `backend/src/risk/engine.rs` |
| Sample positions | `backend/src/risk/portfolio.rs` |
| UW endpoints / parsing | `backend/src/uw/client.rs`, `models.rs` |
| HTTP routes | `backend/src/api/mod.rs` |
| Dashboard layout / copy | `frontend/src/components/Dashboard.tsx` |
| API base URL | `frontend/src/lib/api.ts` + `NEXT_PUBLIC_API_URL` |
| Secrets | `.env` (see `.env.example`) |
