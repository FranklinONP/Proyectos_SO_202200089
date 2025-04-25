package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	pb "go-deployment1/proto"
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

// publishToRabbitMQ simula la publicación en RabbitMQ
func publishToRabbitMQ(ctx context.Context, req *pb.WeatherRequest) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
		log.Printf("Simulando publicación en RabbitMQ: Description=%s, Country=%s, Weather=%s",
			req.Description, req.Country, req.Weather)
		// Aquí iría la lógica real para publicar en RabbitMQ
		return nil
	}
}

// publishToKafka simula la publicación en Kafka
func publishToKafka(ctx context.Context, req *pb.WeatherRequest) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
		log.Printf("Simulando publicación en Kafka: Description=%s, Country=%s, Weather=%s",
			req.Description, req.Country, req.Weather)
		// Aquí iría la lógica real para publicar en Kafka
		return nil
	}
}

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
		req := &pb.WeatherRequest{
			Description: tweet.Description,
			Country:     tweet.Country,
			Weather:     tweet.Weather,
		}

		// Configurar contexto con timeout
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		// Publicar en RabbitMQ
		if err := publishToRabbitMQ(ctx, req); err != nil {
			log.Printf("Error al publicar en RabbitMQ: %v", err)
			c.JSON(http.StatusInternalServerError, ApiResponse{
				Message: "Error al procesar el tweet en RabbitMQ",
			})
			return
		}

		// Publicar en Kafka
		if err := publishToKafka(ctx, req); err != nil {
			log.Printf("Error al publicar en Kafka: %v", err)
			c.JSON(http.StatusInternalServerError, ApiResponse{
				Message: "Error al procesar el tweet en Kafka",
			})
			return
		}

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