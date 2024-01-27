mod autotrader;
mod observations;
mod order;
mod orderbook;
mod recovery;
mod username;
mod pnl;

#[tokio::main]
async fn main() {
    let mut trader = autotrader::AutoTrader::new(
        username::Username::KLiang,
        String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
    );

    trader
        .startup()
        .await
        .expect("Failed to connect to the feed and recover from the latest snapshot");

    loop {
        let _ = trader.refresh_latest_observations().await;
        let _ = trader.poll().await;
    }
}
