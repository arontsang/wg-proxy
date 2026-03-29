use std::sync::Arc;
use futures::{select, SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;
use tokio_smoltcp::device::DeviceCapabilities;
use tokio_smoltcp::smoltcp::phy::Medium;
use tokio_tun::Tun;
use crate::device::functional::FunctionalDevice;

pub struct TunnelDevice;

impl TunnelDevice {

    pub fn new() -> FunctionalDevice {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.max_transmission_unit = 1500;
        capabilities.medium = Medium::Ip;
        capabilities.max_burst_size = Some(100);

        let tun =
            Tun::builder()
                .name("")            // if name is empty, then it is set by kernel.
                //.tap()               // uses TAP instead of TUN (default).
                .packet_info()       // avoids setting IFF_NO_PI.
                .up()                // or set it up manually using `sudo ip link set <tun-name> up`.
                .address("10.11.209.1".parse().unwrap())
                .netmask("255.255.255.0".parse().unwrap())
                .close_on_exec()     // or no_close_on_exec()
                .build()
                .unwrap()
                .pop()
                .unwrap();

        FunctionalDevice::new(async |mut process_incoming, mut out_going| {

            let (mut read_tun, mut write_tun) = tokio::io::split(tun);
            let net_to_tun = async {
                loop {
                    let packet = out_going.next().await.unwrap();
                    write_tun.write(packet.as_slice()).await.unwrap();
                    println!("packet sent");
                }
            };
            let tun_to_net = async {
                let mut buf = [0; 1500];
                loop {
                    let len = read_tun.read(&mut buf).await.unwrap();
                    process_incoming.send(buf[..len].into()).await.unwrap();
                    println!("packet recv");
                }
            };
            let _ = join!(net_to_tun, tun_to_net);
        } ,capabilities)
    }

}