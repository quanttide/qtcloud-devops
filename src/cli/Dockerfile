FROM rust:latest

# System dependencies for native crate builds
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

# Coverage tool
RUN cargo install cargo-llvm-cov

WORKDIR /app
