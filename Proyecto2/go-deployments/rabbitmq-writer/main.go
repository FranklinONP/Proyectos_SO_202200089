package main

import (
	"context"
	"log"

	"github.com/streadway/amqp"
	"google.golang.org/grpc"
	pb "github.com/yourusername/weather-app/proto"
)

type server struct {
	pb.UnimplementedWeatherServiceServer
	conn *amqp.Connection
	ch   *amqp.Channel
}

func (s *server) PublishWeather(ctx context.Context, req *pb.WeatherRequest) (*pb.WeatherResponse, error) {
	body := []byte(req.Description + "|" + req.Country + "|" + req.Weather)
	err := s.ch.Publish(
		"",          // exchange
		"message",   // queue
		false,       // mandatory
		false,       // immediate
		amqp.Publishing{
			ContentType: "text/plain",
			Body:        body,
		})
	if err != nil {
		return &pb.WeatherResponse{Success: false}, err
	}
	return &pb.WeatherResponse{Success: true}, nil
}

func main() {
	conn, err := amqp.Dial("amqp://guest:guest@rabbitmq.weather-app.svc.cluster.local:5672/")
	if err != nil {
		log.Fatalf("Failed to connect to RabbitMQ: %v", err)
	}
	defer conn.Close()

	ch, err := conn.Channel()
	if err != nil {
		log.Fatalf("Failed to open a channel: %v", err)
	}
	defer ch.Close()

	_, err = ch.QueueDeclare(
		"message", // name
		true,      // durable
		false,     // delete when unused
		false,     // exclusive
		false,     // no-wait
		nil,       // arguments
	)
	if err != nil {
		log.Fatalf("Failed to declare a queue: %v", err)
	}

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{conn: conn, ch: ch})

	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	log.Fatal(s.Serve(lis))
}