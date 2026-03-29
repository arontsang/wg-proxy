pub mod acceptor;
pub mod device;
pub mod support;
pub mod tunnel;
use std::net::SocketAddr;
use axum::{
    routing::get,
    Router,
};


use crate::acceptor::wg_acceptor::main_loop as wg_main_loop;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tokio::join!(wg_main_loop(), host_http_trigger());
    Ok(())
}

async fn host_http_trigger() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/Sleep", get(sleep));

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match std::env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    let listen_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    Ok(axum::serve(listener, app).await?)
}

async fn sleep() -> &'static str {
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    "Hello, World!"
}