use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Deserialize, Serialize)]
struct WeatherData {
    description: String,
    country: String,
    weather: String,
}

#[post("/input")]
async fn receive_weather(data: web::Json<WeatherData>) -> impl Responder {
    let client = Client::new();
    let go_api_url = "http://go-api-service.weather-app.svc.cluster.local:8080/process";
    
    match client.post(go_api_url).json(&data).send().await {
        Ok(_) => HttpResponse::Ok().json("Data forwarded to Go API"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(receive_weather)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}