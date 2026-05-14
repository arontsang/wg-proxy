
use crate::device;
use crate::support::TokioIo;

use crate::tunnel::http::handle_proxy_request;

use base64::{Engine as _, engine::general_purpose};
use std::env;
use ipstack::IpStackStream;

pub async fn main_loop() -> anyhow::Result<()> {
    let local_set = tokio::task::LocalSet::new();
    let mut tun = device::wg_device::WgDevice::new(
        &local_set,
        env::var("WG_PEER_ENDPOINT")?,
        read_key("WG_PEER_KEY")?,
        read_key("WG_PRIVATE_KEY")?
    ).await?;

    println!("wg device is ready");

    local_set.run_until(async {
        loop {
            match tun.accept().await {
                Ok(IpStackStream::Tcp(tcp)) => {
                    let io = TokioIo::new(tcp);
                    handle_proxy_request(io);
                }
                Ok(IpStackStream::Udp(_)) => {
                    println!("Unable to accept udp");
                }
                Err(err) => {
                    println!("Error accepting connection {}", err);
                }
                _ => {
                    println!("Unable to accept unknown transport");

                }
            }
        }
    }).await    
}

fn read_key(key: &str) -> anyhow::Result<[u8; 32]> {
    let ret = env::var(&key)?;
    let ret = general_purpose::STANDARD.decode(ret)?;

    let ret: [u8; 32] = ret
        .try_into()
        .map_err(|_| anyhow::anyhow!(format!("{key} must decode to exactly 32 bytes")))?;
    Ok(ret)
}