FROM rust:1.81 AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /usr/src/app/target/release/rust-api /usr/local/bin/rust-api
EXPOSE 8080
CMD ["rust-api"]