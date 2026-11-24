# Multi-stage build for TheresavDB Server
FROM rust:1.80-bullseye AS builder

WORKDIR /usr/src/theresavdb
COPY . .

RUN cargo build --release -p theresav_server -p theresav_cli

# Production runtime container
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run with dedicated non-root user
RUN useradd -m -u 1000 -U -s /bin/bash theresav

RUN mkdir -p /var/lib/theresav/data && chown -R theresav:theresav /var/lib/theresav

COPY --from=builder /usr/src/theresavdb/target/release/theresav_cli /usr/local/bin/theresav-cli

USER theresav
WORKDIR /var/lib/theresav

EXPOSE 7432
VOLUME ["/var/lib/theresav/data"]

CMD ["theresav-cli", "--embedded", "/var/lib/theresav/data"]
