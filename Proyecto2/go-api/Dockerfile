# Stage 1: Build the Go binary
FROM golang:1.20 AS builder
WORKDIR /app/api-rest
# Copy the entire project directory
COPY . .
# Download dependencies
RUN go mod download
# Build the binary (main.go is directly in api-rest/)
RUN CGO_ENABLED=0 GOOS=linux go build -o go-api ./main.go

# Stage 2: Create a lightweight runtime image
FROM alpine:latest
# Copy the binary from the builder stage
COPY --from=builder /app/api-rest/go-api /go-api
# Expose port 8080
EXPOSE 8080
# Run the binary
CMD ["/go-api"]