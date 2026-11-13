# Risk Watcher PoC

Proof-of-concept for **Autonomous Risk Watcher** — a position-aware monitoring service that maps options portfolios to Unusual Whales microstructure data and computes a 0–100 risk score with clear drivers.

**Full system walkthrough:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Layout

```
risk-watcher-poc/
├── backend/          # Rust engine (Tokio + Axum + reqwest)
│   └── src/
│       ├── uw/       # Unusual Whales API client (live)
│       ├── risk/     # Live scoring & explanation engine
│       ├── ingest/   # Simulated Kafka-like consumer (stub)
│       └── api/      # HTTP API for the dashboard
├── frontend/         # Next.js dark-mode financial dashboard
└── docs/
    └── ARCHITECTURE.md
```

## Prerequisites

- Rust 1.85+ (`rustup`)
- Node 20+ (frontend)
- An Unusual Whales API token in `.env` (see `.env.example`)

## Quick start

```bash
# Backend (http://127.0.0.1:8080)
cd backend
cargo run -- --probe SPY   # optional connectivity check
cargo run

# Frontend (http://127.0.0.1:3000) — separate terminal
cd frontend
NEXT_PUBLIC_API_URL=http://127.0.0.1:8080 npm run dev -- -p 3000
```

## What’s live vs sample

| Piece | Status |
|--------|--------|
| UW GEX, flow, spot, greeks, earnings | Live |
| Risk scores + drivers | Computed live from UW |
| Portfolio positions | Sample book (no brokerage sync yet) |

## Notes

- API auth: `Authorization: Bearer <token>` → `https://api.unusualwhales.com`
- Keep real keys in `.env` only (gitignored)
