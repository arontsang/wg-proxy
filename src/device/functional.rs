

use lazy_static::lazy_static;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::{Receiver, Sender};
use tokio_smoltcp::device::{AsyncDevice, DeviceCapabilities, Packet};
use tokio_smoltcp::smoltcp::phy::{Medium};

pub struct FunctionalDevice {
    send_queue: Sender<Packet>,
    recv_queue: Receiver<Packet>,
    poller: tokio::task::JoinHandle<()>,
    capabilities: CAPABILITIES,
}


lazy_static! {
    static ref CAPABILITIES: DeviceCapabilities = {
        let mut ret = DeviceCapabilities::default();
        ret.medium = Medium::Ip;
        ret.max_transmission_unit = 1380;
        ret
    };
}

impl FunctionalDevice {
    pub fn new<Poller, F>(poll: Poller, capabilities: CAPABILITIES) -> Self
    where Poller: FnOnce(Sender<Packet>, Receiver<Packet>) -> F,
          F: Future<Output = ()> + Send + 'static {
        let (send_enqueue, send_dequeue) = futures_channel::mpsc::channel::<Packet>(1024);
        let (recv_enqueue, recv_dequeue) = futures_channel::mpsc::channel::<Packet>(1024);
        let task = tokio::spawn(poll(recv_enqueue, send_dequeue));
        Self {
            poller: task,
            send_queue: send_enqueue,
            recv_queue: recv_dequeue,
            capabilities,
        }
    }
}

impl Drop for FunctionalDevice {
    fn drop(&mut self) {
        self.poller.abort();
    }
}

impl futures_core::stream::Stream for FunctionalDevice {
    type Item = std::io::Result<Packet>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.recv_queue.poll_next_unpin(cx) {
            Poll::Ready(Some(packet)) => Poll::Ready(Some(Ok(packet))),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}

impl futures_sink::Sink<Packet> for FunctionalDevice {
    type Error = std::io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.send_queue.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => {
                let err = Error::new(std::io::ErrorKind::Other, err);
                Poll::Ready(Err(err))
            },
            Poll::Pending => Poll::Pending
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        self.send_queue.start_send(item).map_err(|err| {
            let err = Error::new(std::io::ErrorKind::Other, err);
            err
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.send_queue.poll_flush_unpin(cx).map_err(|err| {
            Error::new(std::io::ErrorKind::Other, err)
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.send_queue.poll_close_unpin(cx).map_err(|err| {
            Error::new(std::io::ErrorKind::Other, err)
        })
    }
}

impl AsyncDevice for FunctionalDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
}