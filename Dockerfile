FROM rust:1.71-bookworm as builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./

RUN cargo fetch

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

COPY --from=builder /build/target/release/controller .

CMD ["/app/controller"]