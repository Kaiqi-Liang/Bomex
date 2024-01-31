mod arbitrage;
mod autotrader;
mod book;
mod feed;
mod observations;
mod order;
mod types;
mod username;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    autotrader::AutoTrader::new(
        username::Username::KLiang,
        String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
    )
    .startup()
    .await?;
    Ok(())
}
