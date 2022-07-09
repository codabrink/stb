# syntax=docker/dockerfile:1
FROM rust:1.61
ENV EMBEDDER_HOST=embedder
ENV QDRANT_HOST=qdrant
EXPOSE 8080
RUN apt-get update && apt-get install -y cmake
WORKDIR /usr/src/stb
COPY . .
RUN cargo install --path .
CMD ["stb", "-s"]
