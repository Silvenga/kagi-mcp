# Stage 1: Chef
FROM rust:1-trixie AS chef
RUN cargo install cargo-chef --locked
RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Stage 2: Planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p kagi-mcp

# Stage 4: Runtime
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates libsqlite3-0 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kagi-mcp /usr/local/bin/kagi-mcp
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/kagi-mcp"]
CMD ["--transport", "streamable-http", "--bind", "0.0.0.0:3000"]
