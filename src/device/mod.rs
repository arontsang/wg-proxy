use lazy_static::lazy_static;
use std::io::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use boringtun::noise::Tunn;
use futures::StreamExt;
use tokio_smoltcp::device::{AsyncDevice, DeviceCapabilities, Packet};
use tokio_smoltcp::smoltcp::phy::{Medium};

struct WireGuardDevice {
    send_queue: futures_channel::mpsc::Sender<Packet>,
    recv_queue: futures_channel::mpsc::Receiver<Packet>,
}


lazy_static! {
    static ref CAPABILITIES: DeviceCapabilities = {
        let mut ret = DeviceCapabilities::default();
        ret.medium = Medium::Ip;
        ret.max_transmission_unit = 1380;
        ret
    };
}

impl WireGuardDevice {
    pub fn new() -> Self {
        let (send_enqueue, send_dequeue) = futures_channel::mpsc::channel::<Packet>(1024);
        let (recv_enqueue, recv_dequeue) = futures_channel::mpsc::channel::<Packet>(1024);
        Self {
            send_queue: send_enqueue,
            recv_queue: recv_dequeue,
        }
    }
}

impl futures_core::stream::Stream for WireGuardDevice {
    type Item = std::io::Result<Packet>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.recv_queue.poll_next_unpin(cx) {
            Poll::Ready(Some(packet)) => Poll::Ready(Some(Ok(packet))),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}

impl futures_sink::Sink<Packet> for WireGuardDevice {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

impl AsyncDevice for WireGuardDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &CAPABILITIES
    }
}