FROM rust:1.91-bookworm AS builder

RUN apt-get update && apt-get install -y musl-tools cmake && \
    rustup target add aarch64-unknown-linux-musl

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --target aarch64-unknown-linux-musl

FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/parmail /parmail

ENTRYPOINT ["/parmail"]
CMD ["lambda"]
