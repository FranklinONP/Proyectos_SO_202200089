Para subir imagenes a harbor
    docker build -t 34.135.173.147.nip.io/proyecto2/go-deployment1:latest .
    docker push 34.135.173.147.nip.io/proyecto2/go-deployment1:latest

Para subir los .yml al cluster
    kubectl apply -f nombredel.yaml


Para probar la api rust solita 
    curl http://localhost:8080/health


Para probar el go-deployment1
    go run main.go en el directorio do-deployment1

    Para api-rest
        curl http://localhost:8080/health

        curl http://localhost:8080/status
    
    Para el gRPC
        curl -X POST http://localhost:8080/process \
-H "Content-Type: application/json" \
-d '{"description":"Está lloviendo","country":"GT","weather":"Lluvioso"}'


namespaces   proyecto2