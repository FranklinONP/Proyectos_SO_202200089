from locust import HttpUser, task, between
import random
import json

class WeatherUser(HttpUser):
    wait_time = between(1, 5)

    @task
    def send_weather_data(self):
        weather_types = ["Lluvioso", "Nublado", "Soleado"]
        countries = ["GT", "US", "MX", "CA", "BR"]
        descriptions = ["Está lloviendo", "Cielo despejado", "Nublado con viento"]

        payload = {
            "description": random.choice(descriptions),
            "country": random.choice(countries),
            "weather": random.choice(weather_types)
        }

        self.client.post("/input", json=payload)