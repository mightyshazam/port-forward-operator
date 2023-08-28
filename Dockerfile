FROM rust:1.71-bookworm as builder
ARG VERSION
WORKDIR /build
RUN if [[ "$TARGETARCH" = "arm" ]] ; then echo -n "aarch64" > data  ; else echo -n "x86_64" > data ; fi
RUN cat data
RUN wget https://github.com/mightyshazam/port-forward-operator/releases/download/${VERSION}/controller-`cat data`-unknown-linux-musl.tar.gz \
    && tar -xvf controller-`cat data`-unknown-linux-musl.tar.gz

FROM alpine:3.17
WORKDIR /app
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

COPY --from=builder /build/controller .

CMD ["/app/controller"]