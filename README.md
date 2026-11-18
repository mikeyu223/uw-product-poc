# Unusual Whales Product PoCs

Futures order flow + trading journal (+ earnings IV / 0DTE sweeps) on real Unusual Whales data.

| Mode | What it is |
|------|------------|
| **Futures Order Flow** | Live ES/NQ tape, CVD, POC/VA, Globex/RTH, OI, **native Futures GEX heatmap** (SPX→ES / NDX→NQ) |
| **Trading Journal** | CSV import, expectancy metrics, TOD heatmaps, UW flow/GEX enrichment on fills |
| **Earnings IV Crush** | Front-end vol into earnings |
| **0DTE Sweep Clusters** | Same-day sweep storms from Live Flow |

## Public deploy (no visitor API key)

Visitors never paste an Unusual Whales key. The key lives only in **server env** (`UNUSUAL_WHALES_API_KEY`) on the host.

This repo’s `vercel.json` runs:

- **frontend** — Next.js
- **backend** — Rust container (Dockerfile)
- Rewrites `/api/*` and `/health` → backend

Set in the Vercel project (Production + Preview):

```
UNUSUAL_WHALES_API_KEY=<your server-side UUID>
```

Do **not** set `NEXT_PUBLIC_API_URL` on Vercel (leave empty so the browser calls same-origin `/api/...` and Vercel rewrites to the backend).

## Local

```bash
# backend
cd backend && cargo run

# frontend (separate terminal)
cd frontend
NEXT_PUBLIC_API_URL=http://127.0.0.1:8080 npm run dev -- -p 3000
```

Copy `.env.example` → `.env` with your key (gitignored).

## API (backend)

| Path | Description |
|------|-------------|
| `GET /health` | Health + feature list |
| `GET /api/v1/futures/contracts` | Front-month ES/NQ/RTY/MES/MNQ |
| `GET /api/v1/futures/{contract}/cvd` | Tape + CVD + POC/VA + Globex/RTH + GEX heatmap |
| `GET /api/v1/futures/oi-matrix` | OI vs volume regimes |
| `GET /api/v1/journal/analytics` | Journal KPIs + filters |
| `POST /api/v1/journal/import` | CSV import + UW flow enrich |
| `GET /api/v1/options/earnings-iv` | Earnings IV crush |
| `GET /api/v1/options/sweep-clusters` | 0DTE sweep clusters |

## Pitch

**Futures (Matt / infra):** Rust/Kafka/ClickHouse-ready tape — CVD, blocks, auction levels, and UW GEX on the index proxy, not invented indicators.

**Journal (retention):** TradeZella-style metrics with UW-only flow alignment + dealer GEX on every fill.
