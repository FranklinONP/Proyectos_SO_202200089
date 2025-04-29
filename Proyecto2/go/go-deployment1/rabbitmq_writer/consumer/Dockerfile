# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY rabbitmq_writer/consumer/ .
COPY rabbitmq_writer/message.pb.go rabbitmq_writer/message_grpc.pb.go ./
RUN CGO_ENABLED=0 GOOS=linux go build -o rabbitmq-consumer consumer.go

# Etapa de runtime
FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/rabbitmq-consumer /app/rabbitmq-consumer

CMD ["/app/rabbitmq-consumer"]