use std::ops::Deref;
use tcp_ip::{IpStack, IpStackConfig, IpStackRecv, IpStackSend};

pub struct FunctionalDevice {
    ip_stack: IpStack,

    pub poller: tokio::task::JoinHandle<()>,
}

impl FunctionalDevice {
    pub fn new<TPoller, TFuture>(config: IpStackConfig, poller: TPoller) -> anyhow::Result<Self> where
        TPoller : FnOnce(IpStackSend, IpStackRecv) -> TFuture + 'static + Send,
        TFuture : Future<Output = ()> + Send + 'static,{
        let (ip_stack, ip_stack_send, ip_stack_recv) =
            tcp_ip::ip_stack(config)?;
        let poller = tokio::task::spawn(async move {
            poller(ip_stack_send, ip_stack_recv).await;
        });

        Ok(Self{
            ip_stack,
            poller
        })
    }
}
impl Drop for FunctionalDevice {
    fn drop(&mut self) {
        self.poller.abort();
    }
}

impl Deref for FunctionalDevice {
    type Target = IpStack;
    fn deref(&self) -> &Self::Target {
        &self.ip_stack
    }
}