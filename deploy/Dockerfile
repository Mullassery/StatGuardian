FROM rust:1.89-bookworm AS rust
FROM python:3.12-slim AS builder

COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
ENV PATH="/usr/local/cargo/bin:${PATH}" \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo

RUN apt-get update && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN pip install --no-cache-dir maturin
COPY . /src
WORKDIR /src
RUN maturin build --release --out /wheels

FROM python:3.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /wheels/*.whl /tmp/
RUN pip install --no-cache-dir /tmp/*.whl "fastapi>=0.110" "uvicorn>=0.27" pandas
COPY deploy/contracts /contracts
EXPOSE 8000
HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8000/healthz || exit 1
ENTRYPOINT ["statguardian", "serve", "--contract", "/contracts/orders.sg", "--host", "0.0.0.0", "--port", "8000"]
