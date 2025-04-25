# Etapa de compilación
FROM rust:1.86 AS builder

WORKDIR /usr/src/rust-api
COPY . .
RUN cargo build --release

# Etapa de runtime
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /usr/src/rust-api/target/release/rust-api /app/rust-api

# Instalar dependencias mínimas
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Exponer el puerto
EXPOSE 8080

# Comando para ejecutar la API
CMD ["rust-api"]