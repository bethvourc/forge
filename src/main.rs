use clap::Parser;
use forge::config::CliArgs;
use forge::shared::error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = CliArgs::parse();
    let runtime = forge::app::bootstrap::bootstrap(cli).await?;
    runtime.run().await
}
