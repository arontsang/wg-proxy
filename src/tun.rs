pub mod device;

use tokio_smoltcp::NetConfig;
use tokio_smoltcp::smoltcp::iface::Config;
use tokio_smoltcp::smoltcp::wire::{HardwareAddress, IpCidr};
use device::tun::TunnelDevice;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tun = TunnelDevice::new();
    let config = NetConfig::new(
        Config::new(HardwareAddress::Ip),
        IpCidr::new("10.11.209.100".parse().unwrap(), 24),
        vec![]
    );
    let net = tokio_smoltcp::Net::new(tun, config);
    
    let mut listener = net.tcp_bind("10.11.209.100:3128".parse().unwrap()).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("new connection from {}", addr);
    }

    tokio::time::sleep(std::time::Duration::from_mins(60)).await;
    Ok(())
}