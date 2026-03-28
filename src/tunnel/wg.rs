use tokio_stream::StreamExt;
use anyhow::Result;
use std::env;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tokio_stream::Stream;
use tokio_wireguard::{config::{Config, Interface, Peer}, interface::ToInterface, TcpListener, TcpStream};
use tokio_wireguard::x25519::{PublicKey, StaticSecret};
use crate::support::TokioIo;
use crate::tunnel::http::handle_proxy_request;

fn read_key(key: &str) -> Result<[u8; 32]> {
    let ret = env::var(&key)?;
    let ret = BASE64.decode(ret)?;

    let ret: [u8; 32] = ret
        .try_into()
        .map_err(|_| anyhow::anyhow!(format!("{key} must decode to exactly 32 bytes")))?;
    Ok(ret)
}

pub fn read_private_key(key: &str) -> Result<StaticSecret> {
    Ok(StaticSecret::from(read_key(key)?))
}

pub fn read_pub_key(key: &str) -> Result<PublicKey> {
    Ok(PublicKey::from(read_key(key)?))
}

pub async fn run_tunnel() -> Result<()> {
    let config = Config {
        interface: Interface {
            private_key: read_private_key("WG_PRIVATE_KEY")?,
            // Our address on the WireGuard network
            address: env::var("WG_PRIVATE_ADDRESS")?.parse()?,
            // Let the interface pick a random port
            listen_port: None,
            // Let the interface pick an appropriate MTU
            mtu: None,
        },
        peers: vec![Peer {
            public_key: read_pub_key("WG_PEER_KEY")?,
            // This is where the tunneled WireGuard traffic will be sent
            endpoint: Some(env::var("WG_PEER_ENDPOINT")?.parse()?),
            // IP addresses the peer can handle traffic to and from on the WireGuard network
            // The /32 suffix indicates that the peer only handles traffic for itself
            allowed_ips: vec![env::var("WG_PEER_ADDRESS")?.parse()?],
            // Send a keepalive packet every 15 seconds
            persistent_keepalive: Some(1),
        }],
    };

    let interface = config.to_interface().await?;


    let listener = TcpListener::bind(
        SocketAddr::from(([0, 0, 0, 0], 3128)),
        interface,
    ).await?;

    let listener = TcpListenerStream { tcp_listener: listener };
    let mut listener = listener.map(TokioIo::new);

    handle_proxy_request(&mut listener).await
}

struct TcpListenerStream {
    tcp_listener: TcpListener
}

impl Stream for TcpListenerStream {
    type Item = TcpStream;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.tcp_listener.poll_accept(cx) {
            Poll::Ready(Ok((stream, _))) => Poll::Ready(Some(stream)),
            Poll::Ready(Err(_)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}


