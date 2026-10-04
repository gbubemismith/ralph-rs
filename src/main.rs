use anyhow::{Ok, Result};

use ralph_rs::{
    cli::Cli,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = get_subscriber("info".into());
    init_subscriber(subscriber);

    Cli::start_execution().await?;
    Ok(())
}
