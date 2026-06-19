FROM rust:1-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.lock ./
COPY server/Cargo.toml ./
COPY server/src src/
COPY server/prompts prompts/

RUN cargo build --release \
  && cp /app/target/release/silvie-server /silvie-server

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
COPY /server/config ./config


EXPOSE 8080

ENTRYPOINT ["silvie-server"]
