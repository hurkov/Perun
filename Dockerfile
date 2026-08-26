# ---- Stage 1: builder ----
FROM rust:1.95-slim-bookworm AS builder

# ALSA + system toolchain required to compile rodio's alsa-sys binding.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libasound2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for layer caching, then the sources.
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release \
    && strip --strip-all target/release/perun

# ---- Stage 2: runtime ----
FROM debian:bookworm-slim

# Only the ALSA runtime library is needed at run time.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libasound2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/perun /usr/local/bin/perun

# Run as root for now so the /var/lib/perun volume is writable.
ENV PERUN_BIND=0.0.0.0:3030 \
    PERUN_DATA_DIR=/var/lib/perun \
    RUST_LOG=info

EXPOSE 3030

CMD ["perun"]
