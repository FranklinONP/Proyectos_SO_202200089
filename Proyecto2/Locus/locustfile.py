from locust import HttpUser, task, between, events

class WeatherAPIUser(HttpUser):
    wait_time = between(1, 2)
    total_requests = 0

    @task
    def post_weather_tweet(self):
        if self.total_requests < 3:
            self.client.post("/input", json={
                "description": "Está lloviendo",
                "country": "GT",
                "weather": "Lluvioso"
            })
            self.total_requests += 1
            if self.total_requests == 3:
                self.environment.runner.quit()

    def on_start(self):
        self.total_requests = 0