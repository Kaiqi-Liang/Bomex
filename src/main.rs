use reqwest::Error;
mod autotrader;
mod book;
mod observations;
mod order;
mod recovery;
mod username;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let trader = autotrader::AutoTrader::new(
        username::Username::KLiang,
        String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
        String::from("sytev070"),
    );
    trader.startup().await?;
    trader.refresh_latest_observations().await?;
    Ok(())
}
