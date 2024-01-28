mod autotrader;
mod book;
mod feed;
mod observations;
mod order;
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

    let mut exceptions = 0;
    loop {
        let result = trader.refresh_latest_observations().await;
        if result.is_err() {
            exceptions += 1;
        }

        let result = trader.poll().await;
        if result.is_err() {
            exceptions += 1;
        }

        for (product, book) in trader.books.iter() {
            let credit = 10;
            let (best_bid, best_ask) = book.bbo();
            let result = trader
                .place_order(
                    product,
                    best_bid.map_or(types::Price(1000), |price| price.price + credit),
                    types::Side::Buy,
                    types::Volume(20),
                    order::OrderType::Day,
                )
                .await;
            if result.is_err() {
                exceptions += 1;
            }
            let result = trader
                .place_order(
                    product,
                    best_ask.map_or(types::Price(5000), |price| price.price - credit),
                    types::Side::Sell,
                    types::Volume(20),
                    order::OrderType::Day,
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
