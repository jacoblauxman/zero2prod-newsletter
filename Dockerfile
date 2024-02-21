# Builder stage
FROM rust:1.76.0 AS builder
# latest stable Rust release as base image

# switching working dir to `app` (`cd app`) -> `app` will be created by Docker if it doesn't exist
WORKDIR /app
# install required sys dependencies for linking config
RUN apt update && apt install lld clang -y
# copy all files from working env to Docker image
COPY . .
# via sqlx prepare, metadata of queries from `cargo build` saved into .sqlx dir
ENV SQLX_OFFLINE true
# build binary
RUN cargo build --release

# Runtime stage
FROM rust:1.76.0 AS Runtime
# Copy compiled bin from builder env to runtime env
WORKDIR /app
COPY --from=builder /app/target/release/zero2prod zero2prod
# need config file at runtime
COPY configuration configuration
# instructs configuration setup to use `production.yaml` re: host
ENV APP_ENVIRONMENT production
# `docker run` -> launch binary
# ENTRYPOINT ["./target/release/zero2prod"]
ENTRYPOINT ["./zero2prod"]
