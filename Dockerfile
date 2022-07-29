# syntax=docker/dockerfile:1
FROM rust:latest as builder
RUN apt-get update && apt-get install -y cmake
# RUN apk add --no-cache build-base cmake libressl-dev sqlite-dev
WORKDIR /app
COPY . .
RUN cargo build --release

# FROM rust:1.62.0-alpine
FROM debian:buster-slim
# RUN apk add --no-cache gcompat sqlite-libs sqlite sqlite-dev libressl-dev libgcc openssl
RUN apt-get update && apt-get install -y sqlite openssl
WORKDIR /stb
COPY --from=builder /app/target/release/stb .
COPY ./static ./static
COPY ./templates ./templates
COPY ./Rocket.toml ./Rocket.toml
EXPOSE 8080
CMD ["./stb", "-s"]
