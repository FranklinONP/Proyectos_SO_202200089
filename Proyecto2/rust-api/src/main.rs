use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;

// Estructura para el JSON de entrada
#[derive(Deserialize, Serialize, Debug)]
struct WeatherTweet {
    description: String,
    country: String,
    weather: String,
}

// Estructura para la respuesta
#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

// Endpoint para recibir los tweets del clima
async fn input_weather(
    tweet: web::Json<WeatherTweet>,
    client: web::Data<Client>,
) -> impl Responder {
    // Log para verificar que se recibió la petición
    println!("Petición recibida: {:?}", tweet);

    // Validar el campo weather
    let valid_weathers = vec!["Lluvioso", "Nublado", "Soleado"];
    if !valid_weathers.contains(&tweet.weather.as_str()) {
        println!("Error: Tipo de clima inválido: {}", tweet.weather);
        return HttpResponse::BadRequest().json(ApiResponse {
            message: format!("Tipo de clima inválido: {}. Debe ser Lluvioso, Nublado o Soleado", tweet.weather),
        });
    }

    // Obtener la URL del Deployment de Go desde una variable de entorno
    let go_api_url = env::var("GO_API_URL").unwrap_or_else(|_| "http://go-deployment1-service:8080/process".to_string());

    // Enviar la petición al Deployment de Go
    match client
        .post(&go_api_url)
        .json(&tweet)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("Petición enviada exitosamente al Deployment de Go");
                HttpResponse::Ok().json(ApiResponse {
                    message: "Tweet procesado correctamente".to_string(),
                })
            } else {
                println!("Error en la respuesta del Deployment de Go: {}", response.status());
                HttpResponse::InternalServerError().json(ApiResponse {
                    message: "Error al procesar el tweet en el Deployment de Go".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error al enviar la petición al Deployment de Go: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse {
                message: format!("Error al enviar la petición: {}", e),
            })
        }
    }
}

// Endpoint para verificar que la API está funcionando
async fn health_check() -> impl Responder {
    println!("Health check solicitado");
    HttpResponse::Ok().json(ApiResponse {
        message: "API Rust funcionando correctamente".to_string(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configurar el puerto desde una variable de entorno o usar 8080 por defecto
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let address = format!("0.0.0.0:{}", port);

    println!("Iniciando servidor en {}", address);

    // Crear el cliente HTTP para reenviar peticiones
    let client = Client::new();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .route("/process", web::post().to(input_weather))
            .route("/health", web::get().to(health_check))
    })
    .bind(&address)?
    .run()
    .await
}