use reqwest::Error;
mod autotrader;
mod book;
mod username;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let trader = autotrader::AutoTrader {
        username: username::Username::KLiang,
        password: String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
        host: String::from("sytev070"),
    };
    trader.startup().await?;
    trader.refresh_latest_observations().await?;
    Ok(())
}
