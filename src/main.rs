mod autotrader;
mod feed;
mod observations;
mod order;
mod orderbook;
mod types;
mod username;

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

    // tokio::spawn(async move {
    //     tokio::signal::ctrl_c().await;
    //     trader.shutdown();
    // });

    loop {
        let _ = trader.refresh_latest_observations().await;
        let _ = trader.poll().await;
    }
}
