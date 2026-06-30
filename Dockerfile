FROM rust:1.96.0-bookworm AS builder

WORKDIR /usr/src/rrdb

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin rrdb

FROM debian:bookworm-slim

ENV RRDB_BASE_PATH=/var/lib/rrdb
ENV RUST_LOG=info

COPY --from=builder /usr/src/rrdb/target/release/rrdb /usr/local/bin/rrdb

RUN mkdir -p /var/lib/rrdb

EXPOSE 22208
VOLUME ["/var/lib/rrdb"]

ENTRYPOINT ["sh", "-c", "rrdb init --base-path \"$RRDB_BASE_PATH\" && exec rrdb run --base-path \"$RRDB_BASE_PATH\""]
