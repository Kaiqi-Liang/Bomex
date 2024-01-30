use crate::{
    book::Book,
    feed::HasSequence,
    order::{AddMessage, BulkDeleteMessage, DeleteMessage, MessageType, OrderType},
    types::{Price, Side, Volume},
    username::Username,
};
use futures_util::stream::{SplitStream, StreamExt};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[macro_export]
macro_rules! url {
    ($port:expr, $endpoint:expr) => {
        format!("http://{}:{}/{}", AutoTrader::HOSTNAME, $port, $endpoint)
    };
    ($protocol:expr, $port:expr, $endpoint:expr) => {
        format!(
            "{}://{}:{}/{}",
            $protocol,
            AutoTrader::HOSTNAME,
            $port,
            $endpoint
        )
    };
}

macro_rules! send_order {
    ($message:expr) => {
        reqwest::Client::new()
            .post(url!(AutoTrader::EXECUTION_PORT, "execution"))
            .json($message)
            .send()
            .await?
            .json()
            .await?
    };
}

macro_rules! get_book {
    ($books:expr, $message:ident) => {
        $books
            .get_mut(&$message.product)
            .expect("Product is not in the books")
    };
}

type Websocket = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub struct AutoTrader {
    pub username: Username,
    pub password: String,
    pub books: HashMap<String, Book>,
    pub sequence: u32,
    pub stream: Option<Websocket>,
}

pub trait ConstantPorts {
    const OBSERVATION_PORT: u16;
    const EXECUTION_PORT: u16;
    const FEED_RECOVERY_PORT: u16;
    const HOSTNAME: &'static str;
}

impl ConstantPorts for AutoTrader {
    const OBSERVATION_PORT: u16 = 8090;
    const EXECUTION_PORT: u16 = 9050;
    const FEED_RECOVERY_PORT: u16 = 9000;
    const HOSTNAME: &'static str = "sytev070";
}

impl AutoTrader {
    pub fn new(username: Username, password: String, stream: Option<Websocket>) -> AutoTrader {
        AutoTrader {
            username,
            password,
            books: HashMap::new(),
            sequence: 0,
            stream,
        }
    }

    pub async fn startup(&mut self) -> Result<(), reqwest::Error> {
        let messages: Vec<crate::feed::Message> =
            reqwest::get(url!(AutoTrader::FEED_RECOVERY_PORT, "recover"))
                .await?
                .json()
                .await?;
        for message in messages {
            self.sequence = message.sequence();
            self.parse_feed_message(message);
        }
        Ok(())
    }

    /// Outbound decoder for feed
    fn parse_feed_message(&mut self, message: crate::feed::Message) {
        // println!("{:#?}", message);
        match message {
            crate::feed::Message::Future(future) => {
                self.books
                    .insert(future.product, Book::new(future.station_id));
            }
            crate::feed::Message::Added(added) => {
                get_book!(self.books, added).add_order(added, &self.username);
            }
            crate::feed::Message::Deleted(deleted) => {
                get_book!(self.books, deleted).remove_order(deleted, &self.username);
            }
            crate::feed::Message::Trade(trade) => {
                get_book!(self.books, trade).trade(trade, &self.username);
            }
            crate::feed::Message::Settlement(settlement) => {
                println!(
                    "Book {} settles at {:?}",
                    settlement.product, settlement.price,
                );
            }
            crate::feed::Message::Index(index) => {
                println!("{index:#?}");
            }
            crate::feed::Message::TradingHalt(halt) => {
                self.books.remove(&halt.product);
            }
        }
    }

    pub async fn poll(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Polling websocket");
        while let Some(message) = self.stream.as_mut().unwrap().next().await {
            let message: crate::feed::Message = serde_json::from_slice(&message?.into_data())?;
            let next_sequence = self.sequence + 1;
            if message.sequence() == next_sequence {
                self.sequence = next_sequence;
                self.parse_feed_message(message);
            } else if message.sequence() > next_sequence {
                panic!(
                    "Expecting sequence number {} but got {}",
                    next_sequence,
                    message.sequence()
                );
            }
        }
        Ok(())
    }

    pub async fn place_order(
        &self,
        product: &str,
        price: Price,
        side: Side,
        volume: Volume,
        order_type: OrderType,
    ) -> Result<(), reqwest::Error> {
        send_order!(&crate::order::Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::Add(AddMessage {
                message_type: MessageType::Add,
                product,
                price,
                side,
                volume,
                order_type,
            }),
        });
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn cancel_order(&self, product: &str, id: &str) -> Result<(), reqwest::Error> {
        send_order!(&crate::order::Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::Delete(DeleteMessage {
                message_type: MessageType::Delete,
                product,
                id,
            })
        });
        Ok(())
    }

    pub async fn cancel_all_orders_in_book(&self, product: &str) -> Result<(), reqwest::Error> {
        println!("Cancelling all orders in book {}", product);
        send_order!(&crate::order::Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::BulkDelete(BulkDeleteMessage {
                message_type: MessageType::BulkDelete,
                product,
            })
        });
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down");
        for id in self.books.keys() {
            self.cancel_all_orders_in_book(id).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        book::{Order, Position, PriceLevel},
        observations::Station,
    };
    use serde_json::{from_value, json};
    use std::collections::BTreeMap;

    macro_rules! parse_json {
        ($trader:ident, $json:tt) => {
            $trader.parse_feed_message(
                from_value(json!($json)).expect("Failed to parse feed message"),
            );
        };
    }

    #[test]
    fn test_parse_feed_message() {
        let mut trader = AutoTrader::new(Username::KLiang, String::new(), None);
        assert_eq!(trader.books, HashMap::new());

        let product = String::from("F_SOP_APP0104T0950");

        parse_json!(trader, {
            "type": "FUTURE",
            "product": product,
            "stationId": 66212,
            "stationName": "SYDNEY OLYMPIC PARK AWS (ARCHERY CENTRE)",
            "expiry": "2024-01-04 09:50+1100",
            "haltTime": "2024-01-04 09:50+1100",
            "unit": "APPARENT_TEMP",
            "strike": 0,
            "aggressiveFee": 0,
            "passiveFee": 0,
            "announcementFee": 0,
            "incentiveRebatePerUnit": 0,
            "maxIncentiveRebate": 0,
            "brokerFee": 0,
            "sequence": 1,
        });

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "1",
            "side": "BUY",
            "price": 24.31,
            "filled": 0,
            "resting": 20,
            "owner": "kliang",
            "sequence": 2
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2431),
                    volume: Volume(20),
                }),
                None
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(20))]),
                    asks: BTreeMap::new(),
                    orders: HashMap::from([(
                        String::from("1"),
                        Order {
                            price: Price(2431),
                            owner: Username::KLiang,
                            volume: Volume(20),
                        }
                    )]),
                    position: Position {
                        bid_exposure: Volume(20),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "2",
            "side": "BUY",
            "price": 24.31,
            "filled": 0,
            "resting": 15,
            "owner": "cchuah",
            "sequence": 3
        });

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "3",
            "side": "SELL",
            "price": 33.29,
            "filled": 0,
            "resting": 20,
            "owner": "cchuah",
            "sequence": 4
        });

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "4",
            "side": "SELL",
            "price": 31.01,
            "filled": 0,
            "resting": 50,
            "owner": "rcurby",
            "sequence": 5
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2431),
                    volume: Volume(35),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(50),
                }),
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(35))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(50))]),
                    orders: HashMap::from([
                        (
                            String::from("1"),
                            Order {
                                price: Price(2431),
                                owner: Username::KLiang,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("2"),
                            Order {
                                price: Price(2431),
                                owner: Username::CChuah,
                                volume: Volume(15),
                            },
                        ),
                        (
                            String::from("3"),
                            Order {
                                price: Price(3329),
                                owner: Username::CChuah,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("4"),
                            Order {
                                price: Price(3101),
                                owner: Username::RCurby,
                                volume: Volume(50),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(20),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        parse_json!(trader, {
            "type": "DELETED",
            "product": product,
            "id": "1",
            "side": "BUY",
            "sequence": 6
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2431),
                    volume: Volume(15),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(50),
                }),
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(15))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(50))]),
                    orders: HashMap::from([
                        (
                            String::from("2"),
                            Order {
                                price: Price(2431),
                                owner: Username::CChuah,
                                volume: Volume(15),
                            },
                        ),
                        (
                            String::from("3"),
                            Order {
                                price: Price(3329),
                                owner: Username::CChuah,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("4"),
                            Order {
                                price: Price(3101),
                                owner: Username::RCurby,
                                volume: Volume(50),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "5",
            "side": "SELL",
            "price": 30.01,
            "filled": 0,
            "resting": 1,
            "owner": "kliang",
            "sequence": 7
        });

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "6",
            "side": "BUY",
            "price": 28.90,
            "filled": 0,
            "resting": 5,
            "owner": "kliang",
            "sequence": 8
        });

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": "7",
            "side": "BUY",
            "price": 29.99,
            "filled": 0,
            "resting": 1,
            "owner": "kliang",
            "sequence": 9
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2999),
                    volume: Volume(1),
                }),
                Some(PriceLevel {
                    price: Price(3001),
                    volume: Volume(1),
                }),
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([
                        (Price(2431), Volume(15)),
                        (Price(2890), Volume(5)),
                        (Price(2999), Volume(1)),
                    ]),
                    asks: BTreeMap::from([
                        (Price(3329), Volume(20)),
                        (Price(3101), Volume(50)),
                        (Price(3001), Volume(1)),
                    ]),
                    orders: HashMap::from([
                        (
                            String::from("2"),
                            Order {
                                price: Price(2431),
                                owner: Username::CChuah,
                                volume: Volume(15),
                            },
                        ),
                        (
                            String::from("3"),
                            Order {
                                price: Price(3329),
                                owner: Username::CChuah,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("4"),
                            Order {
                                price: Price(3101),
                                owner: Username::RCurby,
                                volume: Volume(50),
                            },
                        ),
                        (
                            String::from("5"),
                            Order {
                                price: Price(3001),
                                owner: Username::KLiang,
                                volume: Volume(1),
                            },
                        ),
                        (
                            String::from("6"),
                            Order {
                                price: Price(2890),
                                owner: Username::KLiang,
                                volume: Volume(5),
                            },
                        ),
                        (
                            String::from("7"),
                            Order {
                                price: Price(2999),
                                owner: Username::KLiang,
                                volume: Volume(1),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(6),
                        ask_exposure: Volume(1),
                        position: 0,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": product,
            "price": 30.01,
            "volume": 1,
            "buyer": "pshannon",
            "seller": "kliang",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "5",
            "passiveOrderRemaining": 0,
            "aggressorOrder": "8",
            "sequence": 10
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2999),
                    volume: Volume(1),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(50),
                }),
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([
                        (Price(2431), Volume(15)),
                        (Price(2890), Volume(5)),
                        (Price(2999), Volume(1)),
                    ]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(50))]),
                    orders: HashMap::from([
                        (
                            String::from("2"),
                            Order {
                                price: Price(2431),
                                owner: Username::CChuah,
                                volume: Volume(15),
                            },
                        ),
                        (
                            String::from("3"),
                            Order {
                                price: Price(3329),
                                owner: Username::CChuah,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("4"),
                            Order {
                                price: Price(3101),
                                owner: Username::RCurby,
                                volume: Volume(50),
                            },
                        ),
                        (
                            String::from("6"),
                            Order {
                                price: Price(2890),
                                owner: Username::KLiang,
                                volume: Volume(5),
                            },
                        ),
                        (
                            String::from("7"),
                            Order {
                                price: Price(2999),
                                owner: Username::KLiang,
                                volume: Volume(1),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(6),
                        ask_exposure: Volume(0),
                        position: -1,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        // wash trade
        parse_json!(trader, {
            "type": "TRADE",
            "product": product,
            "price": 29.99,
            "volume": 1,
            "buyer": "kliang",
            "seller": "kliang",
            "tradeType": "SELL_AGGRESSOR",
            "passiveOrder": "7",
            "passiveOrderRemaining": 0,
            "aggressorOrder": "9",
            "sequence": 11
        });

        // leftover volume on orderbook
        parse_json!(trader, {
            "type": "TRADE",
            "product": product,
            "price": 28.90,
            "volume": 2,
            "buyer": "kliang",
            "seller": "kliang",
            "tradeType": "SELL_AGGRESSOR",
            "passiveOrder": "6",
            "passiveOrderRemaining": 3,
            "aggressorOrder": "10",
            "sequence": 12
        });

        assert_eq!(
            trader
                .books
                .get(&product)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2890),
                    volume: Volume(3),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(50),
                }),
            )
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(15)), (Price(2890), Volume(3))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(50))]),
                    orders: HashMap::from([
                        (
                            String::from("2"),
                            Order {
                                price: Price(2431),
                                owner: Username::CChuah,
                                volume: Volume(15),
                            },
                        ),
                        (
                            String::from("3"),
                            Order {
                                price: Price(3329),
                                owner: Username::CChuah,
                                volume: Volume(20),
                            },
                        ),
                        (
                            String::from("4"),
                            Order {
                                price: Price(3101),
                                owner: Username::RCurby,
                                volume: Volume(50),
                            },
                        ),
                        (
                            String::from("6"),
                            Order {
                                price: Price(2890),
                                owner: Username::KLiang,
                                volume: Volume(3),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(3),
                        ask_exposure: Volume(0),
                        position: -1,
                    },
                    station_id: Station::SydOlympicPark,
                },
            )])
        );

        parse_json!(trader, {
            "type": "TRADING_HALT",
            "product": product,
            "sequence": 13
        });

        parse_json!(trader, {
            "type": "SETTLEMENT",
            "product": product,
            "stationName": "SYDNEY OLYMPIC PARK AWS (ARCHERY CENTRE)",
            "expiry": "2024-01-04 09:50+1100",
            "price": 26.05,
            "sequence": 14
        });

        assert_eq!(trader.books, HashMap::new());
    }
}
