


use boringtun::noise::{Tunn, TunnResult};
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use boringtun::x25519::{PublicKey, StaticSecret};
use crate::device::functional::FunctionalDevice;

pub struct WgDevice{
    peer_endpoint: SocketAddr,
    peer_public_key: [u8; 32],
    private_key: [u8; 32],
}


impl WgDevice {

    pub fn new (peer_endpoint: SocketAddr, peer_public_key: [u8; 32], private_key: [u8; 32]) -> Self {
        Self {
            private_key,
            peer_endpoint,
            peer_public_key
        }
    }

    fn build_tunnel(&self) -> Tunn {
        Tunn::new(
            StaticSecret::from(self.private_key.clone()),
            PublicKey::from(self.peer_public_key.clone()),
            None,
            Some(1),
            100,
            None
        )
    }

    pub async fn build(self) -> anyhow::Result<FunctionalDevice> {
        let socket = tokio::net::UdpSocket::bind(SocketAddr::from(([0,0,0,0], 0))).await?;
        socket.connect(&self.peer_endpoint).await?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let wg = WgDevice::build_tunnel(&self);


        let mut net_stack_config = tcp_ip::IpStackConfig::default();
        net_stack_config.mtu = 1380;

        FunctionalDevice::new(net_stack_config, |ip_stack_send, mut ip_stack_recv|async move{
            std::thread::spawn(move || {


                rt.block_on(async {
                    let socket = Rc::new(socket);
                    let wg = Rc::new(RefCell::new(wg));


                    let handle_tunnel_result = {
                        let socket = socket.clone();
                        async move |result: &TunnResult| {
                            match result {
                                TunnResult::WriteToNetwork(buffer) => {
                                    socket.send(buffer).await.unwrap();
                                }
                                TunnResult::Err(_) => {
                                    println!("Error");
                                }
                                TunnResult::Done => { }
                                TunnResult::WriteToTunnelV4(buffer, _) => {
                                    match ip_stack_send.send_ip_packet(buffer).await {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("Error sending ip packet {}", err);
                                        }
                                    }
                                }
                                TunnResult::WriteToTunnelV6(buffer, _) => {
                                    match ip_stack_send.send_ip_packet(buffer).await {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("Error sending ip packet {}", err);
                                        }
                                    }
                                }
                            }
                        }
                    };

                    let tun_to_net = {
                        let socket = socket.clone();
                        let wg = wg.clone();
                        let mut udp_buffer = [0u8; 1500];
                        let mut net_buffer = [0u8; 1500];
                        let handle_tunnel_result = handle_tunnel_result.clone();
                        async move {
                            loop {
                                let len = socket.recv(&mut udp_buffer).await.unwrap();
                                let requires_more_decapsulation = {
                                    let udp_buffer = &udp_buffer[..len];
                                    let result = wg.borrow_mut().decapsulate(None, udp_buffer, &mut net_buffer);
                                    handle_tunnel_result(&result).await;
                                    matches! (result, TunnResult::WriteToNetwork(_))
                                };

                                if requires_more_decapsulation {
                                    loop {
                                        match wg.borrow_mut().decapsulate(None, &[], &mut udp_buffer) {
                                            TunnResult::WriteToNetwork(buffer) => {
                                                socket.send(buffer).await.ok();
                                            }
                                            _ => break
                                        }
                                    }
                                }
                            }
                        }
                    };

                    let net_to_tun = {
                        let wg = wg.clone();
                        let mut udp_buffer = [0u8; 1500];
                        let mut net_buffer = [0u8; 1500];
                        let handle_tunnel_result = handle_tunnel_result.clone();
                        async move {
                            loop {
                                let len = ip_stack_recv.recv(&mut net_buffer).await.unwrap();
                                let net_buffer = &net_buffer[..len];
                                let result = wg.borrow_mut().encapsulate(net_buffer, &mut udp_buffer);
                                handle_tunnel_result(&result).await;
                            }
                        }
                    };

                    let timer = {
                        //let peer_endpoint = peer_endpoint.clone();
                        let socket = socket.clone();
                        let wg = wg.clone();
                        async move {
                            let mut buffer = [0u8; 1500];
                            loop {
                                match wg.borrow_mut().update_timers(&mut buffer) {
                                    TunnResult::WriteToNetwork(buffer) => {
                                        socket.send(buffer.as_ref()).await.ok();
                                    }
                                    _ => { }
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                        }
                    };

                    tokio::join!(tun_to_net, net_to_tun, timer);
                });
            });
        })
    }
}