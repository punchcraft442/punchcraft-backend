# ── Stage 1: dependency cache planner ────────────────────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-1.87-bookworm AS chef
WORKDIR /app

# ── Stage 2: generate the recipe (only Cargo files needed) ───────────────────
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
# src must exist for cargo-chef to resolve the workspace correctly
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: build dependencies then the app ─────────────────────────────────
FROM chef AS builder

# pkg-config + libssl-dev are required to compile native-tls (used by reqwest)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Build dependencies first (cached unless Cargo.toml/Cargo.lock change)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and compile
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Punchcraft-openapi.yaml is embedded at compile time via include_str!
COPY Punchcraft-openapi.yaml ./Punchcraft-openapi.yaml

RUN cargo build --release --bin punchcraft

# ── Stage 4: minimal runtime image ───────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies: TLS certs + OpenSSL (for reqwest native-tls)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

# Non-root user for security
RUN useradd --uid 1001 --no-create-home --shell /bin/false punchcraft

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/punchcraft ./punchcraft

RUN chown punchcraft:punchcraft ./punchcraft

USER punchcraft

# Default port — override BIND_ADDR at runtime if needed
EXPOSE 8080

# All config is injected via environment variables at runtime (no .env in image)
CMD ["./punchcraft"]
