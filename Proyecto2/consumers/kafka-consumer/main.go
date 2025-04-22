package main

import (
    "context"
    "log"
    "strings"
    "sync"

    "github.com/redis/go-redis/v9"
    "github.com/segmentio/kafka-go"
)

func main() {
    redisClient := redis.NewClient(&redis.Options{
        Addr: "redis.tweets-clima.svc.cluster.local:6379",
    })

    reader := kafka.NewReader(kafka.ReaderConfig{
        Brokers:  []string{"kafka.tweets-clima.svc.cluster.local:9092"},
        Topic:    "message",
        GroupID:  "kafka-consumer-group",
        MinBytes: 10e3, // 10KB
        MaxBytes: 10e6, // 10MB
    })
    defer reader.Close()

    var wg sync.WaitGroup
    for i := 0; i < 10; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            consumeMessages(reader, redisClient)
        }()
    }
    wg.Wait()
}

func consumeMessages(reader *kafka.Reader, redisClient *redis.Client) {
    ctx := context.Background()
    for {
        msg, err := reader.ReadMessage(ctx)
        if err != nil {
            log.Printf("Error reading message: %v", err)
            continue
        }

        parts := strings.Split(string(msg.Value), "|")
        if len(parts) != 3 {
            continue
        }
        country := parts[1]

        // Store in Redis (hash table)
        redisClient.HIncrBy(ctx, "country_counts", country, 1)
        redisClient.Incr(ctx, "total_messages")
    }
}