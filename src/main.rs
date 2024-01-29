mod autotrader;
mod book;
mod feed;
mod order;
mod types;
mod username;
use crate::{
    order::OrderType,
    types::{Price, Side, Volume},
};
use autotrader::AutoTrader;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use username::Username;

#[tokio::main]
async fn main() {
    let (stream, response) = connect_async("ws://sytev070:9000/information")
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
    let mut exceptions = 0;
    loop {
        // TODO: don't wait for observations
        let result = trader.refresh_latest_observations().await;
        if result.is_err() {
            exceptions += 1;
        }

        let result = trader.poll().await;
        if result.is_err() {
            exceptions += 1;
        }
        println!("{}", trader.books.len());
        for (product, book) in trader.books.iter() {
            println!("{:#?}", trader.observations.get(&book.station_id));
            let credit = 10;
            let (best_bid, best_ask) = book.bbo();
            println!("{:?}", best_bid);
            println!("{:?}", best_ask);
            let result = trader
                .place_order(
                    product,
                    best_bid.map_or(Price(1000), |price| price.price + credit),
                    Side::Buy,
                    Volume(20),
                    OrderType::Day,
                )
                .await;
            if result.is_err() {
                exceptions += 1;
            }
            let result = trader
                .place_order(
                    product,
                    best_ask.map_or(Price(5000), |price| price.price - credit),
                    Side::Sell,
                    Volume(20),
                    OrderType::Day,
                )
                .await;
            if result.is_err() {
                exceptions += 1;
            }
        }

        if exceptions > 100 {
            break;
        }
    }
    let _ = trader.shutdown().await;
}
