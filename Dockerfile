FROM rust:1-buster as builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config

WORKDIR /usr/src/kitsune

COPY . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update && apt-get install -y libssl-dev pkg-config
COPY --from=builder /usr/src/kitsune/target/release/kitsune /usr/local/bin/kitsune

ENTRYPOINT [ "/usr/local/bin/kitsune" ]