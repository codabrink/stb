# syntax=docker/dockerfile:1
FROM rust:1.61
RUN apt-get update && apt-get install -y cmake
WORKDIR /usr/src/stb
COPY . .
RUN cargo install --path .
EXPOSE 8080
CMD ["stb"]
