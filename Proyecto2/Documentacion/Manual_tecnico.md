# Laboratorio Sistemas Operativos 1

# Primer Semestre

### Primer Semestre 2025

```js
Universidad San Carlos de Guatemala
Programador: Franklin Orlando Noj Pérez
Carne: 202200089
Correo: master11frank@gmail.com/ 3110022770701@ingenieria.usac.edu.gt
```

---

## Este documento describe la implementación de un sistema de mensajería distribuido que utiliza Valkey y Apache Kafka como componentes principales para procesar y almacenar mensajes generados por una API REST. El proyecto tiene como objetivo comparar el rendimiento y la escalabilidad de ambos sistemas, integrando flujos de datos a través de RabbitMQ y Kafka, con visualización en Grafana. Se ha observado que los mensajes llegan más rápido a Valkey, que opera con dos réplicas de consumidores, mientras que Kafka utiliza una sola réplica, lo que sugiere diferencias en la latencia y la capacidad de procesamiento.

## Objetivos

- Objetivo General
  - Administrar una arquitectura en la nube utilizando Kubernetes en Google Cloud Platform
(GCP).
- Objetivos Específicos
  - Utilizar el lenguaje de programación Golang y Rust, maximizando su concurrencia y
aprovechando sus librerías.
  - Crear y desplegar contenedores en un Container Registry y utilizarlo como un medio de
almacenamiento.
  - Entender el funcionamiento de un message broker utilizando Kafka y RabbitMQ.
  - Crear un sistema de alta concurrencia de manejo de mensajes.




## Caracteristicas del programa

- Desarrollado en rust la appi de usuario/deployment
- Uso de habor
- Uso de go para los deployment
- Uso de valkey y kafka
- Visualizacion de datos a traves de grafana.

---
## Tecnologias necesarias

- habor
- GKE
- Go
- Grafana
- Rust
- Locus.py
---


# Comandos para ver el Locust

-  locust -f locustfile.py \
  --host=http://rust-api.34.122.73.35.nip.io \
  --users 100 \
  --spawn-rate 10 \
  --run-time 60s \
  --headless


 ## Descripcion de la Arquitectura del Sistema
La arquitectura se basa en un clúster Kubernetes que alberga los siguientes componentes:

API REST: Desplegada en rust-api.34.122.73.35.nip.io/process, recibe mensajes con datos como descripción, país y clima.
RabbitMQ: Utilizado como intermediario con una cola no duradera (message), procesada por un consumidor que almacena datos en Valkey.
Kafka: Configurado con un topic message y un consumidor que almacena datos en Redis.
Valkey: Base de datos en memoria que almacena contadores por país (country_counts) y total de mensajes (total_messages).
Redis: Similar a Valkey, pero asociado al flujo de Kafka.
Grafana: Paneles para visualizar métricas de Valkey y Kafka en tiempo real (grafana.34.122.73.35.nip.io).

La API genera 10,000 mensajes simulados usando Locust, distribuidos entre los flujos de RabbitMQ/Valkey y Kafka/Redis.

Análisis de Rendimiento
Latencia y Réplicas de Consumidores
Pruebas realizadas indican que los mensajes llegan más rápido a Valkey en comparación con Kafka. Esto puede atribuirse a las siguientes razones:

Valkey con dos réplicas de consumidores: La implementación de consumer groups en Valkey permite repartir la carga entre dos réplicas, lo que reduce el tiempo de procesamiento por consumidor. Valkey utiliza un enfoque de balanceo de carga interno, asignando mensajes a consumidores disponibles sin depender de particiones fijas, lo que mejora la eficiencia en escenarios de baja latencia.
Kafka con una réplica de consumidor: Kafka, por diseño, asigna consumidores a particiones específicas dentro de un grupo de consumidores. Con una sola réplica, el procesamiento se limita a un único consumidor por partición, lo que puede introducir cuellos de botella si el volumen de mensajes excede la capacidad de procesamiento individual.

Durante la prueba con 10,000 mensajes, Valkey alcanzó un total de 6,994 mensajes procesados, mientras que Kafka procesó 6,988, con una diferencia marginal que sugiere que la latencia adicional en Kafka podría deberse a la replicación asíncrona y la falta de réplicas adicionales de consumidores.
Factores de Replicación

Valkey: No utiliza un factor de replicación tradicional como Kafka, sino que depende de la persistencia en memoria y la redundancia a través de múltiples instancias (dos réplicas de consumidores en este caso). Esto minimiza la sobrecarga de sincronización entre réplicas.
Kafka: Configurado con un factor de replicación de 1 para el topic message, lo que asegura alta disponibilidad pero no distribuye la carga de consumo. Aumentar el factor de replicación a 2 o 3 podría mejorar la durabilidad, pero requeriría ajustes en min.insync.replicas y podría incrementar la latencia.


Recomendaciones
Para optimizar el sistema, se sugieren las siguientes acciones:

Aumentar réplicas en Kafka: Considerar escalar a dos o tres réplicas de consumidores en Kafka, ajustando el número de particiones del topic message (por ejemplo, a 2 o 3) y usando kubectl scale para aumentar las réplicas del Deployment de kafka-consumer.
Ajustar configuración de Kafka: Establecer acks=all y min.insync.replicas=2 para garantizar durabilidad sin sacrificar demasiado rendimiento.
Monitoreo continuo: Expandir los paneles de Grafana para incluir métricas de latencia por consumidor y uso de recursos en ambos sistemas.

## Ejemplo de visualizacion en grafana 

![alt text](<Captura desde 2025-05-02 07-35-47.png>)

El proyecto demuestra que Valkey ofrece una latencia más baja con dos réplicas de consumidores, mientras que Kafka, con una sola réplica, presenta un rendimiento ligeramente inferior. La escalabilidad y la configuración de réplicas son clave para equilibrar velocidad y durabilidad, sugiriendo que un enfoque híbrido podría maximizar los beneficios de ambos sistemas.


- Autor: Franklin Orlando Noj Perez          
- 202200089
- Facultad de Ingenieria
- Escuela de Ciencias y Sistemas
- USAC
- Fecha: Mayo 02, 2025