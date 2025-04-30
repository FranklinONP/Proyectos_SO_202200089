package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	pbKafka "go-deployment1/kafka_writer"
	pbRabbit "go-deployment1/rabbitmq_writer"
	"google.golang.org/grpc"
)

// WeatherTweet representa el JSON de entrada
type WeatherTweet struct {
	Description string `json:"description" binding:"required"`
	Country     string `json:"country" binding:"required"`
	Weather     string `json:"weather" binding:"required"`
}

// ApiResponse representa la respuesta de la API
type ApiResponse struct {
	Message string `json:"message"`
}

// ValidWeatherTypes lista de climas válidos
var ValidWeatherTypes = []string{"Lluvioso", "Nublado", "Soleado"}

// isValidWeather verifica si el clima es válido
func isValidWeather(weather string) bool {
	for _, w := range ValidWeatherTypes {
		if weather == w {
			return true
		}
	}
	return false
}

func main() {
	// Conectar al gRPC Server (RabbitMQ Writer)
	rabbitConn, err := grpc.Dial("rabbitmq-writer-service:50051", grpc.WithInsecure(), grpc.WithBlock(), grpc.WithTimeout(5*time.Second))
	if err != nil {
		log.Fatalf("Error al conectar al gRPC Server de RabbitMQ: %v", err)
	}
	defer rabbitConn.Close()
	rabbitClient := pbRabbit.NewRabbitMQWriterClient(rabbitConn)

	// Conectar al gRPC Server (Kafka Writer)
	kafkaConn, err := grpc.Dial("kafka-writer-service:50052", grpc.WithInsecure(), grpc.WithBlock(), grpc.WithTimeout(5*time.Second))
	if err != nil {
		log.Fatalf("Error al conectar al gRPC Server de Kafka: %v", err)
	}
	defer kafkaConn.Close()
	kafkaClient := pbKafka.NewKafkaWriterClient(kafkaConn)

	// Configurar el puerto desde una variable de entorno
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	// Inicializar el router de Gin
	r := gin.Default()

	// Endpoint de salud para verificar que el servidor está funcionando
	r.GET("/health", func(c *gin.Context) {
		log.Println("Health check solicitado")
		c.JSON(http.StatusOK, ApiResponse{Message: "API REST funcionando correctamente"})
	})

	// Endpoint para procesar tweets
	r.POST("/process", func(c *gin.Context) {
		var tweet WeatherTweet
		if err := c.ShouldBindJSON(&tweet); err != nil {
			log.Printf("Error al parsear JSON: %v", err)
			c.JSON(http.StatusBadRequest, ApiResponse{Message: "JSON inválido"})
			return
		}

		// Validar el campo weather
		if !isValidWeather(tweet.Weather) {
			log.Printf("Tipo de clima inválido: %s", tweet.Weather)
			c.JSON(http.StatusBadRequest, ApiResponse{
				Message: fmt.Sprintf("Tipo de clima inválido: %s. Debe ser uno de: %v",
					tweet.Weather, ValidWeatherTypes),
			})
			return
		}

		// Crear el mensaje gRPC
		req := &pbRabbit.WeatherTweet{
			Description: tweet.Description,
			Country:     tweet.Country,
			Weather:     tweet.Weather,
		}

		// Configurar contexto con timeout
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		// Publicar en RabbitMQ usando gRPC
		respRabbit, err := rabbitClient.PublishMessage(ctx, req)
		if err != nil {
			log.Printf("Error al publicar en RabbitMQ: %v", err)
			c.JSON(http.StatusInternalServerError, ApiResponse{
				Message: "Error al procesar el tweet en RabbitMQ",
			})
			return
		}
		log.Printf("Respuesta de RabbitMQ Writer: %s", respRabbit.Message)

		// Publicar en Kafka usando gRPC
		respKafka, err := kafkaClient.PublishMessage(ctx, &pbKafka.WeatherTweet{
			Description: req.Description,
			Country:     req.Country,
			Weather:     req.Weather,
		})
		if err != nil {
			log.Printf("Error al publicar en Kafka: %v", err)
			c.JSON(http.StatusInternalServerError, ApiResponse{
				Message: "Error al procesar el tweet en Kafka",
			})
			return
		}
		log.Printf("Respuesta de Kafka Writer: %s", respKafka.Message)

		log.Printf("Tweet procesado correctamente: %+v", tweet)
		c.JSON(http.StatusOK, ApiResponse{Message: "Tweet procesado correctamente"})
	})

	// Endpoint adicional para verificar logs
	r.GET("/status", func(c *gin.Context) {
		log.Println("Status check solicitado")
		c.JSON(http.StatusOK, ApiResponse{Message: "Servidor funcionando. Revisa los logs para detalles."})
	})

	// Iniciar el servidor
	log.Printf("Iniciando API REST en :%s", port)
	if err := r.Run(":" + port); err != nil {
		log.Fatalf("Error al iniciar el servidor: %v", err)
	}
}