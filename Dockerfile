FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --no-create-home --uid 1000 app
COPY --from=builder /app/target/release/vitality_bot /usr/local/bin/
COPY startup.sh /startup.sh
RUN chmod +x /startup.sh
USER app
ENV RUST_LOG=info,vitality_bot=info
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 CMD wget -qO- http://localhost:3000/ || exit 1
CMD ["/startup.sh"]