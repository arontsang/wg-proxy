pub mod acceptor;
pub mod device;
pub mod support;
pub mod tunnel;





#[tokio::main]
async fn main() -> anyhow::Result<()> {
    crate::acceptor::wg_acceptor::main_loop().await
}

