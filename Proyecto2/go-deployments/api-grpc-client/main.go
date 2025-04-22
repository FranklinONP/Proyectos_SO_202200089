package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/gorilla/mux"
	"google.golang.org/grpc"
	pb "github.com/yourusername/weather-app/proto"
)

type WeatherData struct {
	Description string `json:"description"`
	Country    string `json:"country"`
	Weather    string `json:"weather"`
}

func main() {
	rabbitConn, err := grpc.Dial("rabbitmq-writer.weather-app.svc.cluster.local:50051", grpc.WithInsecure())
	if err != nil {
		log.Fatalf("Failed to connect to RabbitMQ writer: %v", err)
	}
	defer rabbitConn.Close()
	rabbitClient := pb.NewWeatherServiceClient(rabbitConn)

	kafkaConn, err := grpc.Dial("kafka-writer.weather-app.svc.cluster.local:50051", grpc.WithInsecure())
	if err != nil {
		log.Fatalf("Failed to connect to Kafka writer: %v", err)
	}
	defer kafkaConn.Close()
	kafkaClient := pb.NewWeatherServiceClient(kafkaConn)

	r := mux.NewRouter()
	r.HandleFunc("/weather", func(w http.ResponseWriter, r *http.Request) {
		var data WeatherData
		if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()

		// Publish to RabbitMQ
		_, err = rabbitClient.PublishWeather(ctx, &pb.WeatherRequest{
			Description: data.Description,
			Country:    data.Country,
			Weather:    data.Weather,
		})
		if err != nil {
			log.Printf("Failed to publish to RabbitMQ: %v", err)
		}

		// Publish to Kafka
		_, err = kafkaClient.PublishWeather(ctx, &pb.WeatherRequest{
			Description: data.Description,
			Country:    data.Country,
			Weather:    data.Weather,
		})
		if err != nil {
			log.Printf("Failed to publish to Kafka: %v", err)
		}

		w.WriteHeader(http.StatusOK)
	}).Methods("POST")

	log.Fatal(http.ListenAndServe(":8080", r))
}