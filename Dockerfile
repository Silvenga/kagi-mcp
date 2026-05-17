FROM rust:1-trixie AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p kagi-mcp

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates libsqlite3-0 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kagi-mcp /usr/local/bin/kagi-mcp
EXPOSE 3000
ENV KAGI_TRANSPORT=streamable-http
ENV KAGI_BIND=0.0.0.0:3000
ENTRYPOINT ["kagi-mcp"]
