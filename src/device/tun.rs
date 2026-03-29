use std::sync::Arc;
use tun_rs::{Configuration};
use crate::device::functional::FunctionalDevice;

pub struct TunnelDevice;

impl TunnelDevice {
    pub fn new() -> anyhow::Result<FunctionalDevice> {
        let mut config = Configuration::default();
        config
            .mtu(1500)
            .address_with_prefix_multi(&[("10.11.209.1", 24)])
            .up();

        let dev = tun_rs::create_as_async(&config)?;
        let dev = Arc::new(dev);
        FunctionalDevice::new(tcp_ip::IpStackConfig::default(), |ip_stack_send, mut ip_stack_recv|async move{

            let tun_to_stack = {
                let dev = dev.clone();
                async move {
                    let mut buffer = [0u8; 1500];
                    loop {
                        let len = dev.recv(&mut buffer).await.unwrap();
                        match ip_stack_send.send_ip_packet(&buffer[..len]).await {
                            Ok(()) => {

                                //println!("Recv packet ");
                            },
                            Err(err) => {
                                println!("Failed to send ip packet to tun: {}", err.kind());
                            }
                        };
                    }
                }
            };

            let stack_to_tun = {
                async move {
                    let dev = dev.clone();
                    let mut buffer = [0u8; 1500];
                    loop {
                        let len = ip_stack_recv.recv(&mut buffer).await.unwrap();
                        match dev.send(&buffer[..len]).await {
                            Ok(_) => {

                                println!("Send packet ");
                            },
                            Err(err) => {
                                println!("Failed to send ip packet to tun: {}", err.kind());
                            }
                        };
                    }
                }
            };

            tokio::join!(tun_to_stack, stack_to_tun);
        })
    }
}