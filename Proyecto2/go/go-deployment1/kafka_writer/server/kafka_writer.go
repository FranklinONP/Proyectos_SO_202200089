package main

import (
    "context"
    "fmt"
    "log"
    "net"

    "github.com/segmentio/kafka-go"
    pb "go-deployment1/kafka_writer"
    "google.golang.org/grpc"
)

type KafkaWriterServer struct {
    pb.UnimplementedKafkaWriterServer
    writer *kafka.Writer
}

func NewKafkaWriterServer() *KafkaWriterServer {
    writer := &kafka.Writer{
        Addr:     kafka.TCP("kafka-service:9092"),
        Topic:    "message",
        Balancer: &kafka.LeastBytes{},
    }
    return &KafkaWriterServer{
        writer: writer,
    }
}

func (s *KafkaWriterServer) PublishMessage(ctx context.Context, req *pb.WeatherTweet) (*pb.PublishResponse, error) {
    log.Printf("Recibido mensaje para publicar en Kafka: Description=%s, Country=%s, Weather=%s",
        req.Description, req.Country, req.Weather)

    msg := fmt.Sprintf("Description=%s, Country=%s, Weather=%s",
        req.Description, req.Country, req.Weather)

    err := s.writer.WriteMessages(ctx,
        kafka.Message{
            Value: []byte(msg),
        },
    )
    if err != nil {
        log.Printf("Error al publicar en Kafka: %v", err)
        return nil, err
    }

    log.Printf("Mensaje publicado en Kafka: %s", msg)
    return &pb.PublishResponse{
        Message: "Mensaje publicado en Kafka exitosamente",
    }, nil
}

func main() {
    lis, err := net.Listen("tcp", ":50052")
    if err != nil {
        log.Fatalf("Error al iniciar el listener: %v", err)
    }

    grpcServer := grpc.NewServer()
    pb.RegisterKafkaWriterServer(grpcServer, NewKafkaWriterServer())

    log.Println("Iniciando Kafka Writer gRPC Server en :50052")
    if err := grpcServer.Serve(lis); err != nil {
        log.Fatalf("Error al iniciar el servidor gRPC: %v", err)
    }
}