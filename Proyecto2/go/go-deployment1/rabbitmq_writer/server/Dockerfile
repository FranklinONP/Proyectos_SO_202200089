# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY ../go.mod ../go.sum ./
RUN go mod download
COPY server/ .
COPY ../message.pb.go ../message_grpc.pb.go ./
RUN CGO_ENABLED=0 GOOS=linux go build -o rabbitmq-writer server.go

# Etapa de runtime
FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/rabbitmq-writer /app/rabbitmq-writer

EXPOSE 50051

CMD ["/app/rabbitmq-writer"]