use crate::{
    book::Book,
    observations::Station,
    order::{AddMessage, MessageType, OrderType},
    types::{Price, Side, Volume},
};
use std::collections::HashMap;

pub fn find_arbs<'a>(books: &HashMap<String, Book>) -> Vec<AddMessage<'a>> {
    // TODO: complete this strategy
    let mut orders = Vec::new();
    let mut indices: HashMap<String, [Option<&Book>; 4]> = HashMap::new();
    for book in books.values() {
        let entry = indices.entry(book.expiry.clone()).or_insert([None; 4]);
        entry[book.station_id as usize] = Some(book);
    }
    for index in indices
        .values()
        .map(|index| index.map(|book| book.expect("Book is empty")))
    {
        // let mut underlying_best_asks = [PriceLevel::default(); 3];
        // let mut underlying_asks = [PriceLevel::default(); 3];
        // let mut underlying_min_volume = Volume(u16::MAX);
        // let mut underlying_index_volume = Volume(u16::MAX);
        let mut can_buy_underlying_sell_index = false;
        let mut buy_underlying_sell_index_iters = index.map(|book| {
            if book.station_id == Station::Index {
                book.bids.iter()
            } else {
                book.asks.iter()
            }
        });
        'outer: loop {
            for iter in buy_underlying_sell_index_iters.iter_mut() {
                if let Some((price, volume)) = iter.next() {
                } else {
                    break 'outer;
                }
            }
        }
        // for book in index {
        //     if book.station_id != Station::Index {
        //         for best_ask in book.asks.iter() {
        //             min_volume = *best_ask.1.min(&min_volume);
        //             underlying_best_asks[book.station_id as usize] = best_ask.into();
        //         }
        //         if underlying_best_asks
        //             .iter()
        //             .find(|price_level| price_level.volume == 0)
        //             .is_some()
        //         {
        //             can_buy_underlying_sell_index = true;
        //             break;
        //         }
        //     }
        // }
        if !can_buy_underlying_sell_index {
            // for best_bid in underlying_best_asks.iter_mut() {
            //     best_bid.volume -= min_volume;
            // }
            orders.push(AddMessage {
                message_type: MessageType::Add,
                product: "",
                price: Price(0),
                side: Side::Buy,
                volume: Volume(1),
                order_type: OrderType::Ioc,
            });
        }
    }
    orders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Position;
    use std::collections::BTreeMap;
    static EXPIRY: &str = "2024-01-04 09:50+1100";
    static PRODUCT1: &str = "1";
    static PRODUCT2: &str = "2";
    static PRODUCT3: &str = "3";
    static PRODUCT4: &str = "4";

    #[test]
    fn test_no_orders() {
        let books = HashMap::new();
        assert_eq!(find_arbs(&books), vec![]);
    }

    #[test]
    fn test_no_best_bid() {
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(1200), Volume(20)), (Price(1100), Volume(11))]),
                    asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([
                        (Price(300), Volume(1)),
                        (Price(400), Volume(2)),
                        (Price(500), Volume(2)),
                        (Price(600), Volume(8)),
                    ]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT4.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT4.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            ),
        ]);
        assert_eq!(find_arbs(&books), vec![]);
    }

    #[test]
    fn test_no_best_ask() {
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(1200), Volume(20)), (Price(1100), Volume(11))]),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
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
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
        ]);
        assert_eq!(find_arbs(&books), vec![]);
    }

    #[test]
    fn test_no_arbs() {
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                    asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
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
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
        ]);
        assert_eq!(find_arbs(&books), vec![]);
    }

    #[test]
    fn test_buy_underlying_sell_index() {
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(1200), Volume(20)), (Price(1100), Volume(11))]),
                    asks: BTreeMap::from([(Price(1300), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
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
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                    asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
        ]);
        assert_eq!(
            find_arbs(&books),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1,
                    price: Price(1200),
                    side: Side::Sell,
                    volume: Volume(3),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2,
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
    fn test_buy_index_sell_underlying() {
        let expiry = String::from("2024-01-04 09:50+1100");
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(20)), (Price(450), Volume(11))]),
                    asks: BTreeMap::from([(Price(850), Volume(2)), (Price(1200), Volume(10))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
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
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(500), Volume(2)), (Price(600), Volume(8))]),
                    asks: BTreeMap::from([(Price(750), Volume(9)), (Price(850), Volume(1))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: expiry.clone(),
                },
            ),
        ]);
        assert_eq!(
            find_arbs(&books),
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

    #[test]
    fn test_multiple_levels_arbs() {
        let books = HashMap::from([
            (
                PRODUCT1.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT1.to_string(),
                    station_id: Station::SydAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT2.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1300), Volume(20))]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT2.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT3.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([
                        (Price(500), Volume(2)),
                        (Price(600), Volume(3)),
                        (Price(700), Volume(5)),
                    ]),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT3.to_string(),
                    station_id: Station::CanberraAirport,
                    expiry: EXPIRY.to_string(),
                },
            ),
            (
                PRODUCT4.to_string(),
                Book {
                    bids: BTreeMap::from([
                        (Price(3500), Volume(1)),
                        (Price(3400), Volume(3)),
                        (Price(3200), Volume(20)),
                        (Price(3000), Volume(3)),
                    ]),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position::default(),
                    product: PRODUCT4.to_string(),
                    station_id: Station::Index,
                    expiry: EXPIRY.to_string(),
                },
            ),
        ]);
        assert_eq!(
            find_arbs(&books),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1,
                    price: Price(1200),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2,
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3,
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4,
                    price: Price(3200),
                    side: Side::Sell,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
            ]
        );
    }
}
