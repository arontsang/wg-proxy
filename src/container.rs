pub mod acceptor;
pub mod device;
pub mod support;
pub mod tunnel;
use axum::{routing::{get}, Router, extract::Query};
use serde::{Deserialize};
use std::net::SocketAddr;
use crate::acceptor::wg_acceptor::main_loop as wg_main_loop;
use crate::support::get_int_from_env;



#[tokio::main(flavor = "current_thread")]
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
        .route("/Sleep", get(sleep));

    let port = get_int_from_env("HTTP_BIND_PORT")
        .unwrap_or(80);

    let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    Ok(axum::serve(listener, app).await?)
}

#[derive(Deserialize)]
struct SleepQuery {
    seconds: u64,
}

async fn sleep(query: Query<SleepQuery>) -> &'static str {
    tokio::time::sleep(std::time::Duration::from_secs(query.seconds)).await;
    "Hello, World!"
}