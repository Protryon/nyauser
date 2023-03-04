FROM rust:1.67.1-buster AS builder
ARG BUILD_MODE=release

WORKDIR /build
RUN apt-get update && apt-get install cmake -y

COPY ./src ./src
COPY ./Cargo.lock .
COPY ./Cargo.toml .
RUN if [ "$BUILD_MODE" = "release" ]; then cargo build --release && mv /build/target/release/nyauser /build/target/nyauser; fi
RUN if [ "$BUILD_MODE" = "debug" ]; then cargo build && mv /build/target/debug/nyauser /build/target/nyauser; fi

FROM debian:buster
WORKDIR /runtime

COPY --from=builder /build/target/nyauser /runtime/nyauser

RUN apt-get update && apt-get install libssl1.1 ca-certificates -y && rm -rf /var/lib/apt/lists/*

ENV NYAUSER_CONFIG=/runtime/config/nyauser.yml

ENTRYPOINT ["/runtime/nyauser"]
