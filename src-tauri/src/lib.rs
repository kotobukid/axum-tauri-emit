use axum::{routing::get, Router};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{sync::mpsc, spawn};
use std::sync::Arc;
use lazy_static::lazy_static;
use parking_lot::RwLock;

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

async fn start_axum_server() {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    let emit_task = spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Some(handle) = GLOBAL_APP_HANDLE.read().as_ref() {
                println!("Sending message to Tauri: {}", message);
                handle.emit_filter("axum_event", message, |_|{true}).unwrap();
                // handle.emit_all("axum_event", message).unwrap();
            }
        }
    });

    let shared_state = Arc::new(AxumState { tx });

    let app = Router::new()
        .route("/", get({
            let state = shared_state.clone();
            move || async move {
                let _ = state.tx.send("Hello from Axum to Tauri!".to_string()).await;
                "Hello from Axum!"
            }
        }));

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