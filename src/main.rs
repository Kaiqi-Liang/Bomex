mod autotrader;
mod observations;
mod order;
mod orderbook;
mod recovery;
mod username;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trader = autotrader::AutoTrader::new(
        username::Username::KLiang,
        String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
    );
    trader
        .startup()
        .await
        .expect("Failed to connect to the feed and recover from the latest snapshot");
    trader.refresh_latest_observations().await?;
    trader.poll().await?;
    Ok(())
}
