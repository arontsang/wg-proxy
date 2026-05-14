use std::cell::RefCell;
use boringtun::noise::{Tunn, TunnResult};
pub use std::cell::UnsafeCell;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::pin::{Pin};
use std::rc::Rc;
use std::task::{Context, Poll};
use boringtun::x25519::{PublicKey, StaticSecret};
use ipstack::{IpStack, IpStackConfig, IpStackError, IpStackStream, TcpConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, ReadHalf, SimplexStream, WriteHalf};
use tokio::net::lookup_host;

use tokio::task::{JoinHandle, LocalSet};
use crate::support::get_value_from_env;

struct WgDeviceProxy {
    read: ReadHalf<SimplexStream>,
    write: WriteHalf<SimplexStream>,
}

impl AsyncRead for WgDeviceProxy {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().read).poll_read(cx, buf)
    }
}

impl AsyncWrite for WgDeviceProxy {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().write).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().write).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().write).poll_shutdown(cx)
    }
}

pub struct WgDevice {
    ip_stack: IpStack,
    poller: JoinHandle<()>
}

impl Drop for WgDevice {
    fn drop(&mut self) {
        self.poller.abort();
    }
}

impl WgDevice {
    pub async fn accept(&mut self) -> Result<IpStackStream, IpStackError> {
        self.ip_stack.accept().await
    }

}

impl WgDevice {
    fn build_tunnel(peer_public_key: [u8; 32], private_key: [u8; 32]) -> Tunn {
        Tunn::new(
            StaticSecret::from(private_key.clone()),
            PublicKey::from(peer_public_key.clone()),
            None,
            Some(1),
            100,
            None
        )
    }

    pub async fn new(local_set: &LocalSet, peer_endpoint: String, peer_public_key: [u8; 32], private_key: [u8; 32]) -> anyhow::Result<WgDevice> {
        let socket = tokio::net::UdpSocket::bind(SocketAddr::from(([0,0,0,0], 0))).await?;
        let mut peer = lookup_host(peer_endpoint).await?;
        let peer = peer.next()
            .ok_or_else(||Error::new(ErrorKind::AddrNotAvailable, "No address found"))?;
        socket.connect(peer).await?;

        let wg = WgDevice::build_tunnel(peer_public_key, private_key);

        let (mut ip_stack_recv, write) = tokio::io::simplex(1500);
        let (read, ip_stack_send) = tokio::io::simplex(1500);

        let proxy = WgDeviceProxy {
            write,
            read
        };

        let ip_stack_send = Rc::new(RefCell::new(ip_stack_send));

        let poller = local_set.spawn_local(async move {
            let socket = Rc::new(socket);
            let wg = Rc::new(UnsafeCell::new(wg));

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
                            match ip_stack_send.borrow_mut().write(buffer).await {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("Error sending ip packet {}", err);
                                }
                            }
                        }
                        TunnResult::WriteToTunnelV6(buffer, _) => {
                            match ip_stack_send.borrow_mut().write(buffer).await {
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
                let mut wg = wg.clone();
                let mut udp_buffer = [0u8; 1500];
                let mut net_buffer = [0u8; 1500];
                let handle_tunnel_result = handle_tunnel_result.clone();
                async move {
                    loop {
                        let len = socket.recv(&mut udp_buffer).await.unwrap();
                        let requires_more_decapsulation = {
                            let udp_buffer = &udp_buffer[..len];
                            let result = unsafe { mut_unchecked(&mut wg) }.decapsulate(None, udp_buffer, &mut net_buffer);
                            handle_tunnel_result(&result).await;
                            matches! (result, TunnResult::WriteToNetwork(_))
                        };

                        if requires_more_decapsulation {
                            loop {
                                match unsafe { mut_unchecked(&mut wg) }.decapsulate(None, &[], &mut udp_buffer) {
                                    TunnResult::WriteToNetwork(buffer) => {
                                        socket.send(buffer).await.ok();
                                    }
                                    _ => break
                                }
                            }
                        }

                        tokio::task::yield_now().await;
                    }
                }
            };

            let net_to_tun = {
                let mut wg = wg.clone();
                let mut udp_buffer = [0u8; 1500];
                let mut net_buffer = [0u8; 1500];
                let handle_tunnel_result = handle_tunnel_result.clone();
                async move {
                    loop {
                        if let Ok(len) = ip_stack_recv.read(&mut net_buffer).await {
                            let net_buffer = &net_buffer[..len];
                            let result = unsafe { mut_unchecked(&mut wg) }.encapsulate(net_buffer, &mut udp_buffer);
                            handle_tunnel_result(&result).await;
                        }

                        tokio::task::yield_now().await;
                    }
                }
            };

            let timer = {
                let socket = socket.clone();
                let mut wg = wg.clone();
                async move {
                    let mut buffer = [0u8; 1500];
                    loop {
                        match unsafe { mut_unchecked(&mut wg) }.update_timers(&mut buffer) {
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

        let mut config = IpStackConfig::default();
        let mut tcp_config = TcpConfig::default();
        if let Some(value) = get_value_from_env("WG_MAX_UNACKED_BYTES") {
            tcp_config.max_unacked_bytes = value;
        }

        if let Some(value) = get_value_from_env("WG_MTU") {
            config.mtu_unchecked(value);
        }
        config.with_tcp_config(tcp_config);
        config.packet_information(false);


        let stack = ipstack::IpStack::new(
            config,
            proxy,
        );


        Ok(
            WgDevice {
                ip_stack: stack,
                poller
            }
        )
    }
}

unsafe fn mut_unchecked<T>(cell: &mut Rc<UnsafeCell<T>>) -> &mut T {
    let ptr = cell.get();
    unsafe { &mut *ptr }
}
