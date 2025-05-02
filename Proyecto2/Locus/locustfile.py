import random
from locust import HttpUser, task, between

# Lista de países y climas válidos
COUNTRIES = ["USA", "GT", "MX", "CR", "RD", "NZ", "ES", "FR", "DE", "IT", "PT", "BR", "AR"]
WEATHERS = ["Lluvioso", "Nublado", "Soleado"]

class WeatherUser(HttpUser):
    # Tiempo de espera entre tareas (en segundos)
    wait_time = between(0.1, 0.5)

    @task
    def send_weather_tweet(self):
        # Generar datos aleatorios
        location_number = random.randint(1, 100)  # Número aleatorio para la descripción
        description = f"Clima en {random.choice(COUNTRIES)} {location_number}"
        country = random.choice(COUNTRIES)
        weather = random.choice(WEATHERS)

        # Crear el payload
        payload = {
            "description": description,
            "country": country,
            "weather": weather
        }

        # Enviar la solicitud POST
        self.client.post(
            "/process",
            json=payload,
            headers={"Content-Type": "application/json"}
        )