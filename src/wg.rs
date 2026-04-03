pub mod acceptor;
pub mod device;
pub mod support;
pub mod tunnel;





#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    crate::acceptor::wg_acceptor::main_loop().await
}

