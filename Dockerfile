FROM rust:1.86-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libgdal-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p elevation-main --bins

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    gdal-bin \
    libgdal32 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/prepare /usr/local/bin/prepare
COPY --from=builder /app/target/release/serve /usr/local/bin/serve

CMD ["/usr/local/bin/serve"]
