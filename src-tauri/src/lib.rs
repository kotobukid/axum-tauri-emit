use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::Json, routing::get, Router};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
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

async fn receive_download_file_info(Json(payload): Json<DownloadFileInfo>) -> impl IntoResponse {
    // 抽出した構造体を利用
    println!("Received URL: {}", payload.url);
    println!("Received Hash: {}", payload.hash);
    println!("Received Remote ID: {}", payload.remote_id);

    // 正常なレスポンスを返す
    (StatusCode::OK, "Download info received")
}

async fn start_axum_server() {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let emit_task = spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Some(handle) = GLOBAL_APP_HANDLE.read().as_ref() {
                println!("Sending message to Tauri: {}", message);
                handle.emit_filter("axum_event", message, |_| true).unwrap();
                // handle.emit_all("axum_event", message).unwrap();
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
                move |Json(payload): Json<DownloadFileInfo>| {
                    let state = state.clone();
                    async move {
                        // `DownloadFileInfo`のデータをログ出力
                        println!(
                            "Received DownloadFileInfo: url={}, hash={}, remote_id={}",
                            payload.url, payload.hash, payload.remote_id
                        );

                        // Tauriにデータを送信
                        if let Err(err) = state
                            .tx
                            .send(format!(
                        "Received file info for processing - URL: {}, Hash: {}, Remote ID: {}",
                        payload.url, payload.hash, payload.remote_id
                    ))
                            .await
                        {
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
