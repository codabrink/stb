# syntax=docker/dockerfile:1
FROM rust:1.61
EXPOSE 8080
RUN apt-get update && apt-get install -y cmake
WORKDIR /code
COPY . .
RUN cargo install --path .
CMD ["stb", "-s"]
