Para subir imagenes a harbor
    docker build -t 34.135.173.147.nip.io/proyecto2/rust-api:latest .
    docker push 34.135.173.147.nip.io/proyecto2/rust-api:latest

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

Cosas a instalar
    curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
    
revisar los pods de go *deployment1
    kubectl logs -n proyecto2 go-deployment1-7cf4b7c6b7-xfvhd



Curl

    - http://go-deployment1.34.122.73.35.nip.io/status
    - curl http://go-deployment1.34.122.73.35.nip.io/health
