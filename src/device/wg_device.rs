


use boringtun::noise::{Tunn, TunnResult};
use std::cell::RefCell;
use std::io::{Error, ErrorKind, IoSliceMut};
use std::net::SocketAddr;
use std::rc::Rc;
use boringtun::x25519::{PublicKey, StaticSecret};
use tokio::net::lookup_host;
use tokio::task::LocalSet;
use udp_socket::{RecvMeta, Transmit, UdpSocket};
use crate::device::functional::FunctionalDevice;
use crate::support::get_int_from_env;

pub struct WgDevice{
    peer_endpoint: String,
    peer_public_key: [u8; 32],
    private_key: [u8; 32],
}


impl WgDevice {

    pub fn new (peer_endpoint: String, peer_public_key: [u8; 32], private_key: [u8; 32]) -> Self {
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

    pub async fn build(self, local_set: &LocalSet) -> anyhow::Result<FunctionalDevice> {
        let socket = UdpSocket::bind(SocketAddr::from(([0,0,0,0], 0)))?;
        let mut peer = lookup_host(&self.peer_endpoint).await?;
        let peer = peer.next()
            .ok_or_else(||Error::new(ErrorKind::AddrNotAvailable, "No address found"))?;


        let wg = WgDevice::build_tunnel(&self);


        let mut net_stack_config = tcp_ip::IpStackConfig::default();
        net_stack_config.mtu = get_int_from_env("WG_MTU")
            .unwrap_or(1380);

        FunctionalDevice::new(net_stack_config, local_set, |ip_stack_send, mut ip_stack_recv|async move{
            let socket = Rc::new(socket);
            let wg = Rc::new(RefCell::new(wg));

            let handle_tunnel_result = {
                let socket = socket.clone();
                async move |result: &TunnResult| {
                    match result {
                        TunnResult::WriteToNetwork(buffer) => {
                            socket.send(buffer).await.unwrap();
                        }
                        TunnResult::Err(err) => {
                            println!("WG Error {:?}", err);
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
                let mut net_buffer = [0u8; 1500];
                let handle_tunnel_result = handle_tunnel_result.clone();
                const MAX_BUFFER_SIZE: usize = 1500;
                const MAX_BUFFER_COUNT: usize = 8;
                async move {
                    loop {
                        let io_buffer = [[0u8; MAX_BUFFER_SIZE]; MAX_BUFFER_COUNT];
                        let io_buff_ptr = io_buffer.as_mut_ptr();

                        let meta = [RecvMeta::default(); MAX_BUFFER_COUNT];
                        let meta_ptr = meta.as_mut_ptr();

                        let foo = std::future::poll_fn(move |cx| {
                            let io_buff_ptr = io_buff_ptr;
                            let meta_ptr = meta_ptr;
                            let io_buffer: &mut [[u8; MAX_BUFFER_SIZE]; MAX_BUFFER_COUNT] = unsafe {
                                std::slice::from_raw_parts_mut(io_buff_ptr, MAX_BUFFER_COUNT)
                            }.try_into().unwrap();
                            let mut meta: &mut [RecvMeta] = unsafe {
                                std::slice::from_raw_parts_mut(meta_ptr, MAX_BUFFER_COUNT)
                            };
                            let mut io_buffer = io_buffer.map(|mut x| IoSliceMut::new(&mut x));
                            socket.poll_recv(cx, &mut io_buffer, &mut meta)
                        }).await;

                        let mut requires_more_decapsulation = false;
                        while let Some((udp_buffer, meta)) = io_buffer.iter().zip(meta).next() {
                            if meta.len == 0 { continue; }
                            let udp_buffer = &udp_buffer[..meta.len];
                            let result = wg.borrow_mut().decapsulate(None, udp_buffer, &mut net_buffer);
                            handle_tunnel_result(&result).await;
                            requires_more_decapsulation = requires_more_decapsulation | matches! (result, TunnResult::WriteToNetwork(_))
                        }





                        if requires_more_decapsulation {
                            loop {
                                let mut udp_buffer = [0u8; 1500];
                                match wg.borrow_mut().decapsulate(None, &[], &mut udp_buffer) {
                                    TunnResult::WriteToNetwork(buffer) => {
                                        let payload = Transmit::
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
        })
    }
}