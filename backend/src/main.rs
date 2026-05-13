#[tokio::main]
async fn main() -> anyhow::Result<()> {
    reverie_api::run().await
}

