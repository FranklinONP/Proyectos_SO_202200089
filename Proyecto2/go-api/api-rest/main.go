package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/gorilla/mux"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	pb "github.com/your-repo/Proyecto2/go-api/api-rest/proto"
)

type WeatherData struct {
	Description string `json:"description"`
	Country     string `json:"country"`
	Weather     string `json:"weather"`
}

func processWeather(w http.ResponseWriter, r *http.Request) {
	var data WeatherData
	if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Connect to Kafka gRPC server
	kafkaConn, err := grpc.Dial("kafka-grpc-service.tweets-clima.svc.cluster.local:50051", grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer kafkaConn.Close()
	kafkaClient := pb.NewWeatherServiceClient(kafkaConn)

	// Connect to RabbitMQ gRPC server
	rabbitConn, err := grpc.Dial("rabbitmq-grpc")