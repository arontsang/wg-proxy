pub mod acceptor;
pub mod device;
pub mod support;
pub mod tunnel;
use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use crate::acceptor::wg_acceptor::main_loop as wg_main_loop;
use crate::support::get_int_from_env;

#[derive(Deserialize)]
struct InvokeRequest {
    Data: serde_json::Value,
    Metadata: serde_json::Value,
}

#[derive(Serialize)]
struct InvokeResponse {
    Outputs: Option<serde_json::Value>,
    Logs: Vec<String>,
    ReturnValue: Option<serde_json::Value>,
}

async fn timer_handler(Json(payload): Json<InvokeRequest>) -> Json<InvokeResponse> {
    println!("Timer triggered!");

    tokio::time::sleep(std::time::Duration::from_mins(1)).await;

    Json(InvokeResponse {
        Outputs: None,
        Logs: vec!["Rust timer function executed successfully".to_string()],
        ReturnValue: None,
    })
}

#[tokio::main]
async fn main() {
    match futures_lite::future::or(host_http_trigger(), wg_main_loop()).await {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}", e)
        },
    }
}

async fn host_http_trigger() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/TimerTriggerRust", post(timer_handler))
        .route("/api/Sleep", get(sleep));

    let port = get_int_from_env("FUNCTIONS_CUSTOMHANDLER_PORT")
        .unwrap_or(3000);

    let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    Ok(axum::serve(listener, app).await?)
}

async fn sleep() -> &'static str {
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    "Hello, World!"
}