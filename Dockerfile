# latest stable Rust release as base image
FROM rust:1.76.0

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
# `docker run` -> launch binary
ENTRYPOINT ["./target/release/zero2prod"]
