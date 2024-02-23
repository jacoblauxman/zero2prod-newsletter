# # Builder stage
# FROM rust:1.76.0 AS builder
# # latest stable Rust release as base image

# # switching working dir to `app` (`cd app`) -> `app` will be created by Docker if it doesn't exist
# WORKDIR /app
# # install required sys dependencies for linking config
# RUN apt update && apt install lld clang -y
# # copy all files from working env to Docker image
# COPY . .
# # via sqlx prepare, metadata of queries from `cargo build` saved into .sqlx dir
# ENV SQLX_OFFLINE true
# # build binary
# RUN cargo build --release

# # Runtime stage - bare OS image vs rust toolchain + machinery
# FROM debian:bookworm-slim AS Runtime
# WORKDIR /app
# # install OpenSSL and ca-certificates (verifies TLS certs when establishing HTTP conn's)
# RUN apt-get update -y \
#     && apt-get install -y --no-install-recommends openssl ca-certificates \
#     # clean up
#     && apt-get autoremove -y \
#     && apt-get clean -y \
#     && rm -rf /var/lib/apt/lists/*

# # Copy compiled bin from builder env to runtime env
# COPY --from=builder /app/target/release/zero2prod zero2prod
# # need config file at runtime
# COPY configuration configuration
# # instructs configuration setup to use `production.yaml` re: host
# ENV APP_ENVIRONMENT production
# # `docker run` -> launch binary
# # ENTRYPOINT ["./target/release/zero2prod"]
# ENTRYPOINT ["./zero2prod"]



# utilizing cargo-chef for speeding up container build - runs before actual source code is copied


FROM lukemathwalker/cargo-chef:latest-rust-1.72.0 as chef
# FROM lukemathwalker/cargo-chef:latest-rust-1.76.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
# Build our project
RUN cargo build --release --bin zero2prod

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]
