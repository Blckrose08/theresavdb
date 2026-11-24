FROM rust:bookworm AS builder

WORKDIR /usr/src/theresavdb
COPY . .

RUN cargo build --release -p theresav_cli

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -U -s /bin/bash theresav

RUN mkdir -p /var/lib/theresav/data && chown -R theresav:theresav /var/lib/theresav

COPY --from=builder /usr/src/theresavdb/target/release/theresav-cli /usr/local/bin/theresav-cli

USER theresav
WORKDIR /var/lib/theresav

EXPOSE 7432
VOLUME ["/var/lib/theresav/data"]

CMD ["theresav-cli", "--host", "0.0.0.0", "--port", "7432"]
