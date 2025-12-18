# Build stage
FROM rust:1.91.1-slim as builder

WORKDIR /app

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Copy proto files
COPY proto ./proto

# Copy source code
COPY src ./src

# Build both binaries
RUN cargo build --release --bin server
RUN cargo build --release --bin client

# Server runtime stage
FROM debian:bookworm-slim as server

WORKDIR /app

# Install CA certificates for TLS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /app/server

EXPOSE 50051

CMD ["/app/server"]

# Client runtime stage (REST API Gateway)
FROM debian:bookworm-slim as client

WORKDIR /app

# Install CA certificates for TLS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/client /app/client

EXPOSE 8080

CMD ["/app/client"]