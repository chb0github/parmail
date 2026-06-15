FROM rust:1.91-bookworm AS builder

RUN apt-get update && apt-get install -y musl-tools cmake && \
    rustup target add aarch64-unknown-linux-musl

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --target aarch64-unknown-linux-musl

# Image 1: parmail/interpreter
FROM scratch AS interpreter

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/parmail-interpreter /interpreter

ENTRYPOINT ["/interpreter"]
CMD ["lambda"]

# Image 2: parmail/confirmer
FROM scratch AS confirmer

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/parmail-confirmer /confirmer

ENTRYPOINT ["/confirmer"]
CMD ["lambda"]
