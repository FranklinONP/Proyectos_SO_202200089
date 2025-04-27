# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY . .
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o go-deployment1 main.go
RUN chmod +x go-deployment1

# Etapa de runtime
FROM alpine:latest

# Instalar ca-certificates para conexiones HTTPS
RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/go-deployment1 /app/go-deployment1

# Exponer el puerto
EXPOSE 8080

# Comando para ejecutar la aplicación
CMD ["./go-deployment1"]