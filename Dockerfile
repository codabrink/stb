# syntax=docker/dockerfile:1
FROM rust:1.61.0-alpine as build
RUN apk add --no-cache build-base cmake libressl-dev sqlite-dev
WORKDIR /code
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache gcompat sqlite-libs libgcc
WORKDIR /stb
COPY --from=build /code/target/release/stb .
COPY ./static ./static
COPY ./templates ./templates
EXPOSE 8080
CMD ["./stb", "-s"]
