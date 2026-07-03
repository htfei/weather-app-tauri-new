// 在 release 模式下隐藏 Windows 控制台黑框窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
struct WeatherData {
    location: String,
    temperature: f64,
    description: String,
    humidity: f64,
    wind_speed: f64,
    feels_like: f64,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    name: String,
    main: MainData,
    weather: Vec<WeatherInfo>,
    wind: WindData,
}

#[derive(Debug, Deserialize)]
struct MainData {
    temp: f64,
    humidity: f64,
    feels_like: f64,
}

#[derive(Debug, Deserialize)]
struct WeatherInfo {
    description: String,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f64,
}

#[command]
async fn get_weather(city: String) -> Result<WeatherData, String> {
    let api_key = std::env::var("OPENWEATHER_API_KEY").unwrap_or_else(|_| {
        "demo_key_for_testing".to_string()
    });
    
    if api_key == "demo_key_for_testing" {
        return Ok(WeatherData {
            location: city,
            temperature: 22.5,
            description: "多云".to_string(),
            humidity: 65.0,
            wind_speed: 12.0,
            feels_like: 21.0,
        });
    }
    
    let encoded_city: String = city.chars().map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            c.to_string()
        } else {
            format!("%{:02X}", c as u8)
        }
    }).collect();

    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric&lang=zh_cn",
        encoded_city,
        api_key
    );
    
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    let data: OpenWeatherResponse = response.json().await.map_err(|e| e.to_string())?;
    
    Ok(WeatherData {
        location: data.name,
        temperature: data.main.temp,
        description: data.weather[0].description.clone(),
        humidity: data.main.humidity,
        wind_speed: data.wind.speed * 3.6,
        feels_like: data.main.feels_like,
    })
}

#[command]
async fn get_current_weather() -> Result<WeatherData, String> {
    Ok(WeatherData {
        location: "北京".to_string(),
        temperature: 26.0,
        description: "晴朗".to_string(),
        humidity: 55.0,
        wind_speed: 8.0,
        feels_like: 27.5,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_weather, get_current_weather])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}