// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose, Engine as _};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::Ollama;
use serde_json::{json, Value};
use std::io::Cursor;
use std::{collections::HashMap, env};
use tauri_plugin_store::StoreBuilder;
use xcap::{image, Monitor};

use tauri_plugin_store::Store;

#[derive(Debug, Clone)]
pub struct ClipboardHistory {
    pub text_with_related_tags: HashMap<String, Vec<Value>>,
}

impl ClipboardHistory {
    pub fn load_from_store<R: tauri::Runtime>(
        store: &Store<R>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let text_with_related_tags: HashMap<String, Vec<Value>> = store
            .get("clipboardHistory.text_with_related_tags")
            .and_then(|v| v.as_object().cloned())
            .and_then(|obj| serde_json::from_value(serde_json::Value::Object(obj)).ok())
            .unwrap_or_default();

        Ok(ClipboardHistory {
            text_with_related_tags,
        })
    }
}

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
async fn handle_copy(text: &str, app_handle: tauri::AppHandle) -> Result<String, String> {
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
    let prompt = format!("Analyze the provided information and categorize it into relevant tags as an array (min 10 tags, max 20 tags). Your response should focus solenly on tagging based on both textual content and visual data presented in the screenshots, excluding any other unrelated details or actions. text content: {text}");

    let req = GenerationRequest::new(model, prompt);
    let res = ollama.generate(req.images(images)).await;

    let tags = match res {
        Ok(res) => res.response,
        Err(e) => format!("Error: {e}"),
    };

    println!("{}", tags);

    let mut store = StoreBuilder::new(app_handle, "clipboardHistory.json".into()).build();

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.load();

    let clipboard_history = ClipboardHistory::load_from_store(&store);

    match clipboard_history {
        Ok(clipboard_history) => {
            let mut text_with_related_tags = clipboard_history.text_with_related_tags.clone();

            let tags_vec: Vec<Value> = serde_json::from_str(&tags).unwrap();
            text_with_related_tags.insert(text.to_string(), tags_vec.clone());
            let _ = store.insert(
                "clipboardHistory.text_with_related_tags".to_string(),
                json!(text_with_related_tags),
            );

            let _ = store.save();

            println!("text_with_related_tags {} {:?}", text, tags_vec);

            Ok(tags)
        }
        Err(err) => {
            eprintln!("Error loading settings: {}", err);
            // Handle the error case if needed
            Err(err.to_string()) // Convert the error to a Box<dyn Error> and return Err(err) here
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![greet, handle_copy])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
