# Dockerfile

FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/rust:1.88-slim AS builder

COPY debian.sources /etc/apt/sources.list.d/debian.sources

RUN apt-get update && apt-get install -y \
    protobuf-compiler 

WORKDIR /app

COPY . .

RUN cargo build --release --locked --bin my-cache

FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/ubuntu:20.04

RUN apt-get update && apt-get install -y \
    ca-certificates 

COPY sources.list /etc/apt/sources.list

RUN apt-get update && apt-get install -y \
    libc6

RUN useradd -m -s /bin/bash mycache
USER mycache
WORKDIR /home/mycache/app

COPY --from=builder --chown=mycache:mycache /app/target/release/my-cache .

ENTRYPOINT ["./my-cache"]