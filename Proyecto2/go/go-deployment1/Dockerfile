# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o go-deployment1 main.go

# Etapa de runtime
FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/go-deployment1 /app/go-deployment1

EXPOSE 8080

CMD ["/app/go-deployment1"]