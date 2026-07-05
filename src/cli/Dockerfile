FROM rust:latest

# Accept proxy build args
ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG http_proxy
ARG https_proxy
ARG NO_PROXY
ARG no_proxy

# System dependencies for native crate builds
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

# Coverage tool
RUN cargo install cargo-llvm-cov

WORKDIR /app
