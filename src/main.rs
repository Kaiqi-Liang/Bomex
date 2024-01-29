use std::sync::{Arc, Mutex};
mod autotrader;
mod book;
mod feed;
mod observations;
mod order;
mod types;
mod username;
use crate::observations::refresh_latest_observations;
use crate::username::Username;
use crate::{
    autotrader::ConstantPorts,
    order::OrderType,
    types::{Price, Side, Volume},
};
use autotrader::AutoTrader;
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
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

    println!("Started up with books: {:#?}", trader.books.keys());

    // tokio::spawn(async move {
    //     tokio::signal::ctrl_c().await;
    //     trader.shutdown();
    // });
    let observations = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let observations_update = observations.clone();
        tokio::spawn(async move {
            let _ = refresh_latest_observations(observations_update).await;
        });

        for (product, book) in trader.books.iter() {
            let credit = 10;
            let (best_bid, best_ask) = book.bbo();
            println!("{:?}", best_bid);
            println!("{:?}", best_ask);
            let _ = trader
                .place_order(
                    product,
                    best_bid.map_or(Price(1000), |price| price.price + credit),
                    Side::Buy,
                    Volume(20),
                    OrderType::Day,
                )
                .await;
            let _ = trader
                .place_order(
                    product,
                    best_ask.map_or(Price(5000), |price| price.price - credit),
                    Side::Sell,
                    Volume(20),
                    OrderType::Day,
                )
                .await;
        }
    }
}
