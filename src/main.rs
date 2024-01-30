// use std::sync::Arc;
mod autotrader;
mod book;
mod feed;
mod observations;
mod order;
mod types;
mod username;
use crate::autotrader::ConstantPorts;
// use crate::observations::refresh_latest_observations;
use crate::username::Username;
use autotrader::AutoTrader;
use futures_util::StreamExt;
// use std::collections::HashMap;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let observations = Arc::new(std::sync::Mutex::new(HashMap::new()));
    // let observations_clone = observations.clone();
    // tokio::spawn(async move {
    //     loop {
    //         let result = refresh_latest_observations(observations_clone.clone()).await;
    //         if result.is_err() {
    //             println!("{}", result.err().expect("result.is_err()"));
    //         }
    //     }
    // });

    let (stream, response) =
        connect_async(url!("ws", AutoTrader::FEED_RECOVERY_PORT, "information"))
            .await
            .expect("Failed to connect to the websocket");
    println!("Server responded with headers: {:?}", response.headers());
    let (_, read) = stream.split();

    let mut trader = AutoTrader::new(
        Username::KLiang,
        String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
        Some(read),
    );

    trader
        .startup()
        .await
        .expect("Failed to connect to the feed and recover from the latest snapshot");
    Ok(())
}
