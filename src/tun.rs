pub mod device;
pub mod tunnel;
pub mod support;

use std::net::SocketAddr;
use tcp_ip::tcp::TcpListener;
use device::tun::TunnelDevice;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tun = TunnelDevice::new()?;
    let mut listener = TcpListener::bind_all(tun.clone()).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        let io = support::TokioIo::new(socket);
        println!("new connection from {}", addr);
        tunnel::http::handle_proxy_request(io);
    }

    tokio::time::sleep(std::time::Duration::from_mins(60)).await;
    Ok(())
}