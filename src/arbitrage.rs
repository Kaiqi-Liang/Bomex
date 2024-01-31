use crate::{
    book::{Book, PriceLevel},
    observations::Station,
    order::{AddMessage, MessageType, OrderType},
    types::{Price, Side, Volume},
};
use std::cmp::Ordering;

struct IndexTheo {
    theo: PriceLevel,
    index: PriceLevel,
}

impl IndexTheo {
    fn new() -> Self {
        Self {
            theo: PriceLevel::default(),
            index: PriceLevel::default(),
        }
    }
}

#[derive(PartialEq)]
enum Strategy {
    BuyIndexSellUnderlying,
    BuyUnderlyingSellIndex,
}

fn find_arbs_for_side(index: &[&Book; 4], strategy: Strategy) -> Vec<AddMessage> {
    let mut orders = Vec::new();
    let mut underlying_level = [PriceLevel::default(); 3];
    let mut index_volume = Volume::default();
    let mut underlying_price = [Price::default(); 3];
    let mut index_price = Price::default();
    let mut underlying_volume = Volume::default();
    let mut index_theo = IndexTheo::new();
    let mut book_iters = index.map(|book| {
        let iter: Box<dyn Iterator<Item = (&Price, &Volume)>> =
            if strategy == Strategy::BuyUnderlyingSellIndex && book.station_id == Station::Index
                || strategy == Strategy::BuyIndexSellUnderlying && book.station_id != Station::Index
            {
                Box::new(book.bids.iter().rev())
            } else {
                Box::new(book.asks.iter())
            };
        iter
    });
    'outer: loop {
        let mut underlying_min_volume = Volume::MAX;
        for (i, iter) in book_iters[..3].iter_mut().enumerate() {
            if underlying_level[i].volume == 0 {
                if let Some((&price, &volume)) = iter.next() {
                    let price_level = PriceLevel { price, volume };
                    underlying_level[i] = price_level;
                    underlying_min_volume = underlying_min_volume.min(volume);
                } else {
                    break 'outer;
                }
            } else {
                underlying_min_volume = underlying_min_volume.min(underlying_level[i].volume);
            }
        }
        loop {
            if index_theo.theo.volume == 0 {
                index_theo.theo = PriceLevel {
                    price: underlying_level
                        .iter()
                        .fold(Price::default(), |a, c| a + c.price),
                    volume: underlying_min_volume,
                };
            }
            if index_theo.index.volume == 0 {
                if let Some(index) = book_iters
                    .last_mut()
                    .expect("There are 4 items in book_iters")
                    .next()
                {
                    index_theo.index = index.into();
                } else {
                    break 'outer;
                }
            }
            if index_theo.theo.price.cmp(&index_theo.index.price)
                == if strategy == Strategy::BuyUnderlyingSellIndex {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            {
                index_price = index_theo.index.price;
                let index_min_volume = Volume::min(index_theo.theo.volume, index_theo.index.volume);
                index_volume += index_min_volume;
                index_theo.theo.volume -= index_min_volume;
                index_theo.index.volume -= index_min_volume;
                if index_theo.theo.volume == 0 {
                    underlying_volume += underlying_min_volume;
                    for (i, level) in underlying_level.iter_mut().enumerate() {
                        underlying_price[i] = level.price;
                        level.volume -= underlying_min_volume;
                    }
                    break;
                }
            } else {
                // no more arbs
                break 'outer;
            }
        }
    }
    if index_volume != 0 && index_volume == underlying_volume {
        for (i, price) in underlying_price.into_iter().enumerate() {
            let book = index.get(i).expect("Book does not exist");
            orders.push(AddMessage {
                message_type: MessageType::Add,
                product: book.product.clone(),
                price,
                side: if strategy == Strategy::BuyUnderlyingSellIndex {
                    Side::Buy
                } else {
                    Side::Sell
                },
                volume: underlying_volume,
                order_type: OrderType::Ioc,
            });
        }
        orders.push(AddMessage {
            message_type: MessageType::Add,
            product: index
                .last()
                .expect("There are 4 items in buy_underlying_sell_index_iters")
                .product
                .clone(),
            price: index_price,
            side: if strategy == Strategy::BuyUnderlyingSellIndex {
                Side::Sell
            } else {
                Side::Buy
            },
            volume: index_volume,
            order_type: OrderType::Ioc,
        });
    }
    orders
}

pub fn find_arbs(index: &[&Book; 4]) -> Vec<AddMessage> {
    let mut orders = find_arbs_for_side(index, Strategy::BuyUnderlyingSellIndex);
    if orders.is_empty() {
        orders = find_arbs_for_side(index, Strategy::BuyIndexSellUnderlying);
    }
    orders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Position;
    use std::collections::{BTreeMap, HashMap};
    static EXPIRY: &str = "2024-01-04 09:50+1100";
    static PRODUCT1: &str = "1";
    static PRODUCT2: &str = "2";
    static PRODUCT3: &str = "3";
    static PRODUCT4: &str = "4";

    #[test]
    fn test_no_orders() {
        let books = [
            Book::default(),
            Book::default(),
            Book::default(),
            Book::default(),
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![],
        );
    }

    #[test]
    fn test_no_arbs_on_either_side() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
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
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(1400), Volume(9)), (Price(1350), Volume(2))]),
                asks: BTreeMap::from([(Price(2000), Volume(7))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![],
        );
    }

    #[test]
    fn test_buy_underlying_sell_index_other_side_empty() {
        let books = [
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1300), Volume(20))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT2.to_string(),
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
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
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(1200),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(3200),
                    side: Side::Sell,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_underlying_sell_index_other_side_not_empty() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(800), Volume(100))]),
                asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(800), Volume(100)), (Price(950), Volume(50))]),
                asks: BTreeMap::from([(Price(1300), Volume(20))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT2.to_string(),
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
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
            Book {
                bids: BTreeMap::from([
                    (Price(3500), Volume(1)),
                    (Price(3400), Volume(3)),
                    (Price(3200), Volume(20)),
                    (Price(3000), Volume(3)),
                ]),
                asks: BTreeMap::from([(Price(3599), Volume(99))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(1200),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(3200),
                    side: Side::Sell,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_underlying_sell_index_one_leg_out_of_orders() {
        let books = [
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1300), Volume(20))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT2.to_string(),
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(500), Volume(2)), (Price(600), Volume(3))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
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
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(1200),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(3200),
                    side: Side::Sell,
                    volume: Volume(5),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_underlying_sell_index_index_out_of_volume() {
        let books = [
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1300), Volume(20))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT2.to_string(),
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
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
            Book {
                bids: BTreeMap::from([
                    (Price(3500), Volume(1)),
                    (Price(3400), Volume(3)),
                    (Price(3000), Volume(3)),
                ]),
                asks: BTreeMap::new(),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(1100),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(3400),
                    side: Side::Sell,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_underlying_sell_index_index_out_of_orders() {
        let books = [
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1100), Volume(4)), (Price(1200), Volume(25))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(1300), Volume(20))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT2.to_string(),
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
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
            Book {
                bids: BTreeMap::from([(Price(3500), Volume(1)), (Price(3400), Volume(3))]),
                asks: BTreeMap::new(),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(1100),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(1300),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(600),
                    side: Side::Buy,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(3400),
                    side: Side::Sell,
                    volume: Volume(4),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_index_sell_underlying_enough_volume() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
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
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(900), Volume(9)), (Price(350), Volume(2))]),
                asks: BTreeMap::from([(Price(1200), Volume(100))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(700),
                    side: Side::Sell,
                    volume: Volume(1),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(200),
                    side: Side::Sell,
                    volume: Volume(1),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(500),
                    side: Side::Sell,
                    volume: Volume(1),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(1200),
                    side: Side::Buy,
                    volume: Volume(1),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_index_sell_underlying_big_volume_best_ask() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
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
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(900), Volume(9)), (Price(350), Volume(2))]),
                asks: BTreeMap::from([(Price(1000), Volume(100))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT1.to_string(),
                    price: Price(700),
                    side: Side::Sell,
                    volume: Volume(9),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT2.to_string(),
                    price: Price(100),
                    side: Side::Sell,
                    volume: Volume(9),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT3.to_string(),
                    price: Price(300),
                    side: Side::Sell,
                    volume: Volume(9),
                    order_type: OrderType::Ioc,
                },
                AddMessage {
                    message_type: MessageType::Add,
                    product: PRODUCT4.to_string(),
                    price: Price(1000),
                    side: Side::Buy,
                    volume: Volume(9),
                    order_type: OrderType::Ioc,
                },
            ],
        );
    }

    #[test]
    fn test_buy_index_sell_underlying_index_no_asks() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
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
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::new(),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![],
        );
    }

    #[test]
    fn test_buy_index_sell_underlying_index_no_arbs() {
        let books = [
            Book {
                bids: BTreeMap::from([(Price(700), Volume(20)), (Price(500), Volume(11))]),
                asks: BTreeMap::from([(Price(1000), Volume(5)), (Price(1050), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT1.to_string(),
                station_id: Station::SydAirport,
                expiry: EXPIRY.to_string(),
            },
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
                station_id: Station::SydOlympicPark,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::from([(Price(500), Volume(1)), (Price(300), Volume(8))]),
                asks: BTreeMap::from([(Price(700), Volume(9)), (Price(800), Volume(1))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT3.to_string(),
                station_id: Station::CanberraAirport,
                expiry: EXPIRY.to_string(),
            },
            Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::from([(Price(10000), Volume(5))]),
                orders: HashMap::new(),
                position: Position::default(),
                product: PRODUCT4.to_string(),
                station_id: Station::Index,
                expiry: EXPIRY.to_string(),
            },
        ];
        assert_eq!(
            find_arbs(&[&books[0], &books[1], &books[2], &books[3]]),
            vec![],
        );
    }
}
