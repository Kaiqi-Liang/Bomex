use std::{collections::HashMap, sync::MutexGuard};

use crate::{
    book::Book,
    observations::Station,
    order::{AddMessage, MessageType, OrderType},
    types::{Price, Side, Volume},
};

pub fn find_arbs(books: MutexGuard<'_, HashMap<String, Book>>) -> Vec<AddMessage<'_>> {
    let mut orders = Vec::new();
    let indices: HashMap<String, Vec<&Book>> =
        books.values().fold(HashMap::new(), |mut acc, book| {
            acc.entry(book.expiry.clone())
                .or_default()
                .push(book);
            acc
        });
    for (_, index) in indices {
        let mut underlying_best_bids = Vec::new();
        let mut underlying_best_asks = Vec::new();
        let mut index_best_bid = None;
        let mut index_best_ask = None;
        for book in index {
            let (best_bid, best_ask) = book.bbo();
            if book.station_id == Station::Index {
                index_best_bid = best_bid;
                index_best_ask = best_ask;
            } else {
                underlying_best_bids.push(best_bid);
                underlying_best_asks.push(best_ask);
            }
        }
    }
    orders.push(AddMessage {
        message_type: MessageType::Add,
        product: "",
        price: Price(0),
        side: Side::Sell,
        volume: Volume(20),
        order_type: OrderType::Day,
    });
    orders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Position;
    use std::{collections::BTreeMap, sync::Mutex};

    #[test]
    fn test_no_arbs() {
        let books = Mutex::new(HashMap::new());
        assert_eq!(find_arbs(books.lock().unwrap()), vec![]);
    }

    #[test]
    fn test_buy_underlying_sell_etf() {
        let expiry = String::from("2024-01-04 09:50+1100");
        let books = Mutex::new(HashMap::from([
            (
                String::from("1"),
                Book {
                    bids: BTreeMap::from([(Price(1200), Volume(20)), (Price(1100), Volume(11))]),
                    asks: BTreeMap::from([(Price(1300), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("1"),
                    station_id: Station::Index,
                    expiry: expiry.clone(),
                },
            ),
            (
                String::from("2"),
                Book {
                    bids: BTreeMap::from([(Price(200), Volume(6)), (Price(100), Volume(5))]),
                    asks: BTreeMap::from([
                        (Price(300), Volume(1)),
                        (Price(400), Volume(2)),
                        (Price(500), Volume(2)),
                        (Price(600), Volume(8)),
                    ]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("2"),
                    station_id: Station::SydAirport,
                    expiry: expiry.clone(),
                },
            ),
            (
                String::from("3"),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("3"),
                    station_id: Station::CanberraAirport,
                    expiry: expiry.clone(),
                },
            ),
        ]));
        assert_eq!(
            find_arbs(books.lock().unwrap()),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: "1",
                    price: Price(1200),
                    side: Side::Sell,
                    volume: Volume(3),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "2",
                    price: Price(300),
                    side: Side::Buy,
                    volume: Volume(1),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "2",
                    price: Price(400),
                    side: Side::Buy,
                    volume: Volume(2),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "3",
                    price: Price(700),
                    side: Side::Buy,
                    volume: Volume(3),
                    order_type: OrderType::Ioc,
                },
            ]
        );
    }

    #[test]
    fn test_buy_etf_sell_underlying() {
        let expiry = String::from("2024-01-04 09:50+1100");
        let books = Mutex::new(HashMap::from([
            (
                String::from("1"),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(20)), (Price(450), Volume(11))]),
                    asks: BTreeMap::from([(Price(850), Volume(2)), (Price(1200), Volume(10))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("1"),
                    station_id: Station::Index,
                    expiry: expiry.clone(),
                },
            ),
            (
                String::from("2"),
                Book {
                    bids: BTreeMap::from([(Price(400), Volume(6)), (Price(350), Volume(5))]),
                    asks: BTreeMap::from([
                        (Price(3000), Volume(1)),
                        (Price(4000), Volume(2)),
                        (Price(5000), Volume(2)),
                        (Price(6000), Volume(8)),
                    ]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("2"),
                    station_id: Station::SydAirport,
                    expiry: expiry.clone(),
                },
            ),
            (
                String::from("3"),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(2)), (Price(600), Volume(8))]),
                    asks: BTreeMap::from([(Price(750), Volume(9)), (Price(850), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: String::from("3"),
                    station_id: Station::CanberraAirport,
                    expiry: expiry.clone(),
                },
            ),
        ]));
        assert_eq!(
            find_arbs(books.lock().unwrap()),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: "1",
                    price: Price(850),
                    side: Side::Buy,
                    volume: Volume(2),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "2",
                    price: Price(400),
                    side: Side::Sell,
                    volume: Volume(2),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "2",
                    price: Price(500),
                    side: Side::Sell,
                    volume: Volume(2),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: "3",
                    price: Price(700),
                    side: Side::Sell,
                    volume: Volume(3),
                    order_type: OrderType::Ioc,
                },
            ]
        );
    }
}
