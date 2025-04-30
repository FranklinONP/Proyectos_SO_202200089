# Etapa de compilación
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
# Copiar los archivos generados a un subdirectorio kafka_writer
COPY kafka_writer/kafka_writer.pb.go kafka_writer/kafka_writer_grpc.pb.go kafka_writer/
# Copiar el código principal
COPY kafka_writer/server/kafka_writer.go .
RUN CGO_ENABLED=0 GOOS=linux go build -o kafka-writer kafka_writer.go

# Etapa de runtime
FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/kafka-writer /app/kafka-writer

EXPOSE 50052

CMD ["/app/kafka-writer"]