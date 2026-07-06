#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    weather_app_lib::run();
}