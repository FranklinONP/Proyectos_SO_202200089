# Stage 1: Build the Go binary
FROM golang:1.22 AS builder
WORKDIR /app/api-rest
COPY api-rest/ .
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux go build -o go-api ./main.go

# Stage 2: Create a lightweight runtime image
FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/api-rest/go-api /go-api
EXPOSE 8080
CMD ["/go-api"]