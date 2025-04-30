package main

import (
	"context"
	"fmt"
	"log"
	"strings"
	"sync"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/segmentio/kafka-go"
)

func main() {
	// Conectar a Redis
	redisClient := redis.NewClient(&redis.Options{
		Addr:     "redis-service:6379",
		Password: "",
		DB:       0,
	})
	defer redisClient.Close()

	// Configurar el lector de Kafka
	reader := kafka.NewReader(kafka.ReaderConfig{
		Brokers:  []string{"kafka-service:9092"},
		Topic:    "message",
		GroupID:  "kafka-consumer-group", // Usar un grupo para evitar duplicación
		MinBytes: 10e3,                   // 10KB
		MaxBytes: 10e6,                   // 10MB
	})
	defer reader.Close()

	// Usar un WaitGroup para manejar goroutines
	var wg sync.WaitGroup
	ctx := context.Background()

	// Procesar mensajes con goroutines
	for i := 0; i < 5; i++ { // 5 goroutines para procesamiento concurrente
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			for {
				msg, err := reader.ReadMessage(ctx)
				if err != nil {
					log.Printf("Worker %d: Error al leer mensaje de Kafka: %v", workerID, err)
					continue
				}

				log.Printf("Worker %d: Mensaje recibido de Kafka: %s", workerID, string(msg.Value))

				// Parsear el mensaje
				data := make(map[string]interface{})
				parts := strings.Split(string(msg.Value), ", ")
				for _, part := range parts {
					kv := strings.SplitN(part, "=", 2)
					if len(kv) == 2 {
						data[kv[0]] = kv[1]
					}
				}

				// Almacenar el mensaje como un hash en Redis
				timestamp := time.Now().UnixNano()
				key := fmt.Sprintf("weather_tweet:%d", timestamp)
				err = redisClient.HSet(ctx, key, data).Err()
				if err != nil {
					log.Printf("Worker %d: Error al almacenar el mensaje en Redis: %v", workerID, err)
					continue
				}
				log.Printf("Worker %d: Mensaje almacenado en Redis con clave %s", workerID, key)

				// Incrementar el contador de mensajes por país
				country, exists := data["Country"]
				if !exists {
					log.Printf("Worker %d: No se encontró el campo 'Country' en el mensaje: %v", workerID, data)
					continue
				}
				countryStr, ok := country.(string)
				if !ok {
					log.Printf("Worker %d: El campo 'Country' no es una cadena: %v", workerID, country)
					continue
				}
				err = redisClient.HIncrBy(ctx, "country_counts", countryStr, 1).Err()
				if err != nil {
					log.Printf("Worker %d: Error al incrementar el contador de país en Redis: %v", workerID, err)
					continue
				}
				log.Printf("Worker %d: Contador de país %s incrementado en country_counts", workerID, countryStr)

				// Incrementar el contador total de mensajes
				err = redisClient.HIncrBy(ctx, "total_messages", "count", 1).Err()
				if err != nil {
					log.Printf("Worker %d: Error al incrementar el contador total en Redis: %v", workerID, err)
					continue
				}
				log.Printf("Worker %d: Contador total de mensajes incrementado en total_messages", workerID)
			}
		}(i)
	}

	// Esperar a que las goroutines terminen (esto no debería ocurrir en un consumidor real)
	wg.Wait()
}