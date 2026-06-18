FROM rust:1-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src/ src/

RUN cargo build --release --locked && \
    cp /app/target/release/silvie-server /silvie-server

FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --shell /bin/false --uid 10001 silvie

COPY --from=builder /silvie-server /usr/local/bin/silvie-server

USER silvie
WORKDIR /home/silvie

ENV PORT=8080 \
    RUST_LOG=info,silvie_server=debug

EXPOSE 8080

ENTRYPOINT ["silvie-server"]
