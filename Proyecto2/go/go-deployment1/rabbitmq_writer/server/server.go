package main

import (
    "context"
    "fmt"
    "log"
    "net"

    pb "go-deployment1/rabbitmq_writer"
    "github.com/rabbitmq/amqp091-go"
    "google.golang.org/grpc"
)

type server struct {
    pb.UnimplementedRabbitMQWriterServer
    conn *amqp091.Connection
    ch   *amqp091.Channel
}

func (s *server) PublishMessage(ctx context.Context, tweet *pb.WeatherTweet) (*pb.PublishResponse, error) {
    // Publicar en RabbitMQ
    queueName := "message"
    body := fmt.Sprintf("Description=%s, Country=%s, Weather=%s", tweet.Description, tweet.Country, tweet.Weather)
    err := s.ch.PublishWithContext(
        ctx,
        "",        // exchange
        queueName, // routing key
        false,     // mandatory
        false,     // immediate
        amqp091.Publishing{
            ContentType: "text/plain",
            Body:        []byte(body),
        },
    )
    if err != nil {
        log.Printf("Error al publicar en RabbitMQ: %v", err)
        return nil, err
    }

    log.Printf("Publicado en RabbitMQ: %s", body)
    return &pb.PublishResponse{Message: "Publicado en RabbitMQ correctamente"}, nil
}

func main() {
    // Conectar a RabbitMQ
    conn, err := amqp091.Dial("amqp://guest:guest@rabbitmq-service:5672/")
    if err != nil {
        log.Fatalf("Error al conectar a RabbitMQ: %v", err)
    }
    defer conn.Close()

    ch, err := conn.Channel()
    if err != nil {
        log.Fatalf("Error al abrir canal: %v", err)
    }
    defer ch.Close()

    // Declarar la cola
    _, err = ch.QueueDeclare(
        "message", // nombre
        false,     // durable
        false,     // auto-delete
        false,     // exclusive
        false,     // no-wait
        nil,       // args
    )
    if err != nil {
        log.Fatalf("Error al declarar cola: %v", err)
    }

    // Iniciar servidor gRPC
    lis, err := net.Listen("tcp", ":50051")
    if err != nil {
        log.Fatalf("Error al iniciar servidor gRPC: %v", err)
    }

    s := grpc.NewServer()
    pb.RegisterRabbitMQWriterServer(s, &server{conn: conn, ch: ch})

    log.Println("Servidor gRPC RabbitMQ Writer iniciado en :50051")
    if err := s.Serve(lis); err != nil {
        log.Fatalf("Error al servir gRPC: %v", err)
    }
}