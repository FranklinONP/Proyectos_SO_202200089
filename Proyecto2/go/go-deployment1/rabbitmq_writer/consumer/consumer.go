package main

import (
	"context"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/rabbitmq/amqp091-go"
	"github.com/redis/go-redis/v9"
)

func main() {
	// Función para reintentar conexiones
	retryConnect := func(dialFunc func() (interface{}, error), maxRetries int, delay time.Duration) (interface{}, error) {
		var conn interface{}
		var err error
		for i := 0; i < maxRetries; i++ {
			conn, err = dialFunc()
			if err == nil {
				return conn, nil
			}
			log.Printf("Error al conectar (intento %d/%d): %v", i+1, maxRetries, err)
			time.Sleep(delay)
		}
		return nil, fmt.Errorf("falló después de %d intentos: %v", maxRetries, err)
	}

	// Conectar a RabbitMQ con reintentos
	rabbitConn, err := retryConnect(func() (interface{}, error) {
		return amqp091.Dial("amqp://guest:guest@rabbitmq-service:5672/")
	}, 5, 5*time.Second)
	if err != nil {
		log.Fatalf("Error al conectar a RabbitMQ: %v", err)
	}
	defer rabbitConn.(*amqp091.Connection).Close()

	rabbitCh, err := rabbitConn.(*amqp091.Connection).Channel()
	if err != nil {
		log.Fatalf("Error al abrir canal de RabbitMQ: %v", err)
	}
	defer rabbitCh.Close()

	// Declarar la cola
	q, err := rabbitCh.QueueDeclare(
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

	// Conectar a Valkey con reintentos
	valkeyClient, err := retryConnect(func() (interface{}, error) {
		client := redis.NewClient(&redis.Options{
			Addr:     "valkey-service:6379",
			Password: "",
			DB:       0,
		})
		_, err := client.Ping(context.Background()).Result()
		if err != nil {
			return nil, err
		}
		return client, nil
	}, 5, 5*time.Second)
	if err != nil {
		log.Fatalf("Error al conectar a Valkey: %v", err)
	}
	log.Println("Conectado a Valkey correctamente")

	// Consumir mensajes de RabbitMQ
	msgs, err := rabbitCh.Consume(
		q.Name, // cola
		"",     // consumidor
		true,   // auto-ack
		false,  // exclusive
		false,  // no-local
		false,  // no-wait
		nil,    // args
	)
	if err != nil {
		log.Fatalf("Error al consumir mensajes: %v", err)
	}

	log.Println("Esperando mensajes en la cola 'message'...")

	// Procesar mensajes y enviarlos a Valkey
	ctx := context.Background()
	for msg := range msgs {
		log.Printf("Mensaje recibido de RabbitMQ: %s", msg.Body)

		// Parsear el mensaje (formato: "Description=..., Country=..., Weather=...")
		// Convertimos el mensaje en un mapa para almacenarlo como hash
		data := make(map[string]interface{})
		parts := strings.Split(string(msg.Body), ", ")
		for _, part := range parts {
			kv := strings.SplitN(part, "=", 2)
			if len(kv) == 2 {
				data[kv[0]] = kv[1]
			}
		}

		// Almacenar el mensaje como un hash en Valkey
		timestamp := time.Now().UnixNano()
		key := fmt.Sprintf("weather_tweet:%d", timestamp)
		err = valkeyClient.(*redis.Client).HSet(ctx, key, data).Err()
		if err != nil {
			log.Printf("Error al almacenar el mensaje como hash en Valkey: %v", err)
			continue
		}
		log.Printf("Mensaje almacenado como hash en Valkey con clave %s", key)

		// Incrementar el contador de mensajes por país en la tabla hash "country_counts"
		country, ok := data["Country"].(string)
		if !ok {
			log.Printf("No se pudo obtener el país del mensaje: %v", data)
			continue
		}
		err = valkeyClient.(*redis.Client).HIncrBy(ctx, "country_counts", country, 1).Err()
		if err != nil {
			log.Printf("Error al incrementar el contador de país en Valkey: %v", err)
			continue
		}
		log.Printf("Contador de país %s incrementado en country_counts", country)

		// Incrementar el contador total de mensajes en la tabla hash "total_messages"
		err = valkeyClient.(*redis.Client).HIncrBy(ctx, "total_messages", "count", 1).Err()
		if err != nil {
			log.Printf("Error al incrementar el contador total en Valkey: %v", err)
			continue
		}
		log.Printf("Contador total de mensajes incrementado en total_messages")
	}
}