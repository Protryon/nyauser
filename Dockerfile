FROM lukemathwalker/cargo-chef:0.1.52-rust-1.68.2-slim-buster AS planner
WORKDIR /plan

COPY ./nyauser ./nyauser
COPY ./nyauser-types ./nyauser-types
COPY ./nyauser-cli ./nyauser-cli
COPY ./Cargo.lock .
COPY ./Cargo.toml .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:0.1.52-rust-1.68.2-buster AS builder
ARG BUILD_MODE=release

WORKDIR /build
RUN apt-get update && apt-get install cmake -y

COPY --from=planner /plan/recipe.json recipe.json

COPY ./nyauser ./nyauser
COPY ./nyauser-types ./nyauser-types
COPY ./nyauser-cli ./nyauser-cli
COPY ./Cargo.lock .
COPY ./Cargo.toml .

RUN if [ "$BUILD_MODE" = "release" ]; then cargo chef cook --release -p nyauser && mv /build/target/release/nyauser /build/target/nyauser; fi
RUN if [ "$BUILD_MODE" = "debug" ]; then cargo chef cook -p nyauser && mv /build/target/debug/nyauser /build/target/nyauser; fi

FROM debian:buster-slim
WORKDIR /runtime

COPY --from=builder /build/target/nyauser /runtime/nyauser

RUN apt-get update && apt-get install libssl1.1 ca-certificates -y && rm -rf /var/lib/apt/lists/*

ENV NYAUSER_CONFIG=/runtime/config/nyauser.yml

ENTRYPOINT ["/runtime/nyauser"]
