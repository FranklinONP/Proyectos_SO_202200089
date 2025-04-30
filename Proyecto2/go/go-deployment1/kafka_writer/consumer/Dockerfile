# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app

# 1. Copiar archivos de dependencias primero
COPY go.mod go.sum ./
RUN go mod download

# 2. Copiar código fuente específico
COPY kafka_writer/consumer/kafka_consumer.go .
COPY kafka_writer/kafka_writer.pb.go .
COPY kafka_writer/kafka_writer_grpc.pb.go .

# 3. Compilar la aplicación
RUN CGO_ENABLED=0 GOOS=linux go build -o kafka-consumer kafka_consumer.go

# Etapa de ejecución
FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/kafka-consumer .
RUN chmod +x kafka-consumer
EXPOSE 50051
CMD ["./kafka-consumer"]