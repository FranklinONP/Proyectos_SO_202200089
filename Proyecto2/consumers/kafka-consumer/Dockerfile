FROM golang:1.20 AS builder
WORKDIR /app
COPY . .
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux go build -o kafka-consumer .

FROM alpine:latest
COPY --from=builder /app/kafka-consumer /kafka-consumer
CMD ["/kafka-consumer"]