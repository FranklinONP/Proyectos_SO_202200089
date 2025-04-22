package main

import (
    "context"
    "log"
    "strings"
    "sync"

    "github.com/rabbitmq/amqp091-go"
    "github.com/redis/go-redis/v9" // Using go-redis for Valkey compatibility
)

func main() {
    valkeyClient := redis.NewClient(&redis.Options{
        Addr: "valkey.tweets-clima.svc.cluster.local:6379",
    })

    conn, err := amqp091.Dial("amqp://guest:guest@rabbitmq.tweets-clima.svc.cluster.local:5672/")
    if err != nil {
        log.Fatalf("Failed to connect to RabbitMQ: %v", err)
    }
    defer conn.Close()

    ch, err := conn.Channel()
    if err != nil {
        log.Fatalf("Failed to open a channel: %v", err)
    }
    defer ch.Close()

    q, err := ch.QueueDeclare(
        "message", // name
        true,      // durable
        false,     // auto-deleted
        false,     // exclusive
        false,     // no-wait
        nil,       // arguments
    )
    if err != nil {
        log.Fatalf("Failed to declare queue: %v", err)
    }

    msgs, err := ch.Consume(
        q.Name, // queue
        "",     // consumer
        true,   // auto-ack
        false,  // exclusive
        false,  // no-local
        false,  // no-wait
        nil,    // args
    )
    if err != nil {
        log.Fatalf("Failed to register consumer: %v", err)
    }

 engend   var wg sync.WaitGroup
    for i := 0; i < 10; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            consumeMessages(msgs, valkeyClient)
        }()
    }
    wg.Wait()
}

func consumeMessages(msgs <-chan amqp091.Delivery, valkeyClient *redis.Client) {
    ctx := context.Background()
    for msg := range msgs {
        parts := strings.Split(string(msg.Body), "|")
        if len(parts) != 3 {
            continue
        }
        country := parts[1]

        // Store in Valkey (hash table)
        valkeyClient.HIncrBy(ctx, "country_counts", country, 1)
        valkeyClient.Incr(ctx, "total_messages")
    }
}