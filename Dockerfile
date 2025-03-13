FROM rust:slim-bookworm as builder

WORKDIR /locker

ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV CARGO_INCREMENTAL=0

RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config

COPY . .
RUN cargo build --release --features release ${EXTRA_FEATURES}


FROM debian:bookworm-slim

ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local
ARG BINARY=locker

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq-dev curl procps

EXPOSE 8181

RUN mkdir -p ${CONFIG_DIR}

COPY --from=builder /locker/target/release/${BINARY} ${BIN_DIR}/${BINARY}

WORKDIR ${BIN_DIR}

CMD ./locker

