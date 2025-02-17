use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::spawn;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

async fn start_axum_server() {
    // Axumのルーターを設定
    let app = Router::new().route("/", get(|| async { "Hello from Axum!" }));

    // サーバーのアドレスを設定
    let addr = SocketAddr::from(([127, 0, 0, 1], 30000));

    // サーバーを起動
    println!("Axum server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    spawn(async {
        start_axum_server().await;
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
