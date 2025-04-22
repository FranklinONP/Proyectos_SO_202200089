package main

import (
	"context"
	"log"

	"github.com/segmentio/kafka-go"
	"google.golang.org/grpc"
	pb "github.com/yourusername/weather-app/proto"
)

type server struct {
	pb.UnimplementedWeatherServiceServer
	writer *kafka.Writer
}

func (s *server) PublishWeather(ctx context.Context, req *pb.WeatherRequest) (*pb.WeatherResponse, error) {
	body := req.Description + "|" + req.Country + "|" + req.Weather
	err := s.writer.WriteMessages(ctx,
		kafka.Message{
			Key:   []byte(req.Country),
			Value: []byte(body),
		},
	)
	if err != nil {
		return &pb.WeatherResponse{Success: false}, err
	}
	return &pb.WeatherResponse{Success: true}, nil
}

func main() {
	writer := &kafka.Writer{
		Addr:     kafka.TCP("kafka.weather-app.svc.cluster.local:9092"),
		Topic:    "message",
		Balancer: &kafka.LeastBytes{},
	}
	defer writer.Close()

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{writer: writer})

	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	log.Fatal(s.Serve(lis))
}