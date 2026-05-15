FROM --platform=$BUILDPLATFORM rust:1.91-bookworm AS builder

ARG TARGETARCH

RUN apt-get update && apt-get install -y musl-tools cmake && \
    case "$TARGETARCH" in \
      amd64) RUST_TARGET="x86_64-unknown-linux-musl" ;; \
      arm64) RUST_TARGET="aarch64-unknown-linux-musl" ;; \
      *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add "$RUST_TARGET" && \
    echo "$RUST_TARGET" > /rust_target

RUN case "$TARGETARCH" in \
      amd64) echo "x86_64-linux-musl" > /musl_prefix ;; \
      arm64) \
        apt-get install -y gcc-aarch64-linux-gnu && \
        echo "aarch64-linux-musl" > /musl_prefix ;; \
    esac

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN RUST_TARGET=$(cat /rust_target) && \
    case "$TARGETARCH" in \
      arm64) \
        export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc && \
        export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc ;; \
    esac && \
    cargo build --release --target "$RUST_TARGET" && \
    cp "target/$RUST_TARGET/release/parmail" /parmail

FROM scratch AS app

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /parmail /parmail

ENTRYPOINT ["/parmail"]
CMD ["lambda"]
