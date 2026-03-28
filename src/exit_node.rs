pub mod tunnel;
pub mod support;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tunnel::wg::run_tunnel().await
}
