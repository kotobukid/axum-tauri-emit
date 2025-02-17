use axum::http::StatusCode;
use axum::{extract::Json, routing::get, Router};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::{spawn, sync::mpsc};
use tower_http::cors::{Any, CorsLayer};

// グローバルなAppHandleの保存用
lazy_static! {
    static ref GLOBAL_APP_HANDLE: RwLock<Option<AppHandle>> = RwLock::new(None);
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

struct AxumState {
    tx: mpsc::Sender<String>,
}

#[derive(Serialize, Deserialize)]
struct DownloadFileInfo {
    url: String,
    hash: String,
    remote_id: i64,
}

async fn start_axum_server() {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let emit_task = spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Some(handle) = GLOBAL_APP_HANDLE.read().as_ref() {
                println!("Sending message to Tauri: {}", message);
                handle.emit_filter("axum_event", message, |_| true).unwrap();
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let shared_state = Arc::new(AxumState { tx });

    let app = Router::new()
        .route(
            "/",
            get({
                let state = shared_state.clone();
                move || async move {
                    let _ = state.tx.send("Hello from Axum to Tauri!".to_string()).await;
                    "Hello from Axum!"
                }
            }),
        )
        .route("/health_check", get(|| async { "OK" }))
        .route(
            "/download_file_info",
            axum::routing::post({
                let state = shared_state.clone();
                move |Json(payload): Json<Value>| {
                    let state = state.clone();
                    async move {
                        if let Ok(download_file_info) =
                            serde_json::from_value::<DownloadFileInfo>(payload.clone())
                        {
                            println!(
                                "Parsed DownloadFileInfo: url={}, hash={}, remote_id={}",
                                download_file_info.url,
                                download_file_info.hash,
                                download_file_info.remote_id
                            );
                        }

                        // Tauriにデータを送信
                        if let Err(err) = state.tx.send(payload.to_string()).await {
                            eprintln!("Failed to send message to Tauri: {}", err);
                        }

                        // レスポンスを返す
                        (StatusCode::OK, "Download info received and sent to Tauri")
                    }
                }
            }),
        )
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 30000));
    println!("Axum server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    emit_task.await.unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // AppHandleをグローバルに保存
            let handle = app.handle();
            *GLOBAL_APP_HANDLE.write() = Some(handle.clone());

            tauri::async_runtime::spawn(async {
                start_axum_server().await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
