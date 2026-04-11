# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:bookworm AS builder

# Build dependencies for native-tls (used by reqwest → Cloudinary, Resend, MongoDB)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency compilation separately from application code.
# Copy manifests first — Docker cache invalidates only when these change.
COPY Cargo.toml Cargo.lock ./

# Stubs for both lib and bin targets (Cargo.toml declares both [lib] and [[bin]])
RUN mkdir -p src \
  && echo 'fn main() {}' > src/main.rs \
  && touch src/lib.rs \
  && cargo build --release \
  && rm -rf src

# Now copy the real source and build the application
COPY src ./src
COPY Punchcraft-openapi.yaml ./Punchcraft-openapi.yaml

# Touch main.rs to force Rust to relink (avoids stale cached artifact)
RUN touch src/main.rs && cargo build --release --bin punchcraft

# ── Stage 2: minimal runtime image ───────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Runtime TLS support for outbound HTTPS (Resend, Cloudinary, MongoDB Atlas)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

# Non-root user for security
RUN useradd --uid 1001 --no-create-home --shell /bin/false punchcraft

WORKDIR /app

COPY --from=builder /app/target/release/punchcraft ./punchcraft

RUN chown punchcraft:punchcraft ./punchcraft

USER punchcraft

EXPOSE 8080

CMD ["./punchcraft"]
