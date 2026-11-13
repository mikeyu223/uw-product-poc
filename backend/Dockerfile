# Multi-stage build for Risk Watcher API (Unusual Whales key stays in runtime env).
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/risk_watcher /app/risk_watcher
# Vercel sets PORT at runtime (often 80) and probes that port — do not hardcode BIND_ADDR.
ENV RUST_LOG=risk_watcher=info,tower_http=info
ENV PORT=8080
EXPOSE 8080
CMD ["/app/risk_watcher"]
