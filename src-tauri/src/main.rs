// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose, Engine as _};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::Ollama;
use std::env;
use std::io::Cursor;
use xcap::{image, Monitor};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn greet(name: &str) -> Result<String, ()> {
    let ollama = Ollama::default();

    let model = "llava-phi3:latest".to_string();
    let prompt = format!("You are helpfull AI assistant. {name} want to talk with you.");
    let res = ollama.generate(GenerationRequest::new(model, prompt)).await;

    let text = match res {
        Ok(res) => {
            let response = res.response;
            format!("AI:\n{response}")
        }
        Err(e) => format!("Error: {e}"),
    };
    Ok(text)
}

#[tauri::command]
async fn handle_copy(text: &str) -> Result<String, ()> {
    let monitors = Monitor::all().unwrap();

    let mut images = Vec::new();

    for monitor in monitors {
        println!("capturer {monitor:?}");
        let image = monitor.capture_image().unwrap();

        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("Couldn't write image to bytes.");

        let b64 = general_purpose::STANDARD.encode(bytes);
        images.push(Image::from_base64(&b64));
    }

    let ollama = Ollama::default();

    let model = "llava-phi3:latest".to_string();
    let prompt = format!("Analyze the provided information and categorize it into relevant tags as an array (minimum 10 tags). Your response should focus solenly on tagging based on both textual content and visual data presented in the screenshots, excluding any other unrelated details or actions. text content: {text}");

    let req = GenerationRequest::new(model, prompt);
    let res = ollama.generate(req.images(images)).await;

    let text = match res {
        Ok(res) => {
            let response = res.response;
            format!("clipboard-text: {text}\nAI:\n{response}")
        }
        Err(e) => format!("Error: {e}"),
    };
    Ok(text)
}

fn main() {
    env::set_var("OPENAI_API_BASE_URL", "http://localhost:11434/v1");
    env::set_var("OPENAI_API_KEY", "ollama");

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard::init())
        .invoke_handler(tauri::generate_handler![greet, handle_copy])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
