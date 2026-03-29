pub mod device;
pub mod tunnel;
pub mod support;
use base64::{Engine as _, engine::general_purpose};
use std::env;
use tcp_ip::address::ToSocketAddr;
use tcp_ip::tcp::TcpListener;

fn read_key(key: &str) -> anyhow::Result<[u8; 32]> {
    let ret = env::var(&key)?;
    let ret = general_purpose::STANDARD.decode(ret)?;

    let ret: [u8; 32] = ret
        .try_into()
        .map_err(|_| anyhow::anyhow!(format!("{key} must decode to exactly 32 bytes")))?;
    Ok(ret)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tun = device::wg::WgDevice::new(
        env::var("WG_PEER_ENDPOINT")?.to_addr()?,
        read_key("WG_PEER_KEY")?,
        read_key("WG_PRIVATE_KEY")?
    );

    let tun = tun.build().await?;
    let mut listener = TcpListener::bind_all(tun.clone()).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        let io = support::TokioIo::new(socket);
        println!("new connection from {}", addr);
        tunnel::http::handle_proxy_request(io);
    }
}