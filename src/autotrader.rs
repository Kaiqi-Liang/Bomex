use crate::{
    arbitrage::find_arbs,
    book::Book,
    feed::{HasSequence, Message},
    order::OrderAddedMessage,
    types::Price,
    username::Username,
};
use futures_util::stream::{SplitStream, StreamExt};
use serde_json::{from_slice, from_str, to_string};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpStream, spawn};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

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
    ($username:expr, $password:expr, $message:expr) => {
        reqwest::Client::new()
            .post(url!(AutoTrader::EXECUTION_PORT, "execution"))
            .form(&[
                ("username", $username),
                ("password", &$password),
                (
                    "message",
                    &to_string(&$message).expect("Failed to serializase AddMessage"),
                ),
            ])
            .send()
            .await
    };
}

macro_rules! index_enabled {
    ($index:ident, $enabled_books:ident, $orders_to_wait:ident) => {
        $index.iter().all(|book| {
            *$enabled_books
                .lock()
                .unwrap()
                .get(&book.product)
                .expect("Book does not exist")
                && $orders_to_wait.lock().unwrap().get(&book.product).is_none()
        })
    };
    ($index:ident, $enabled_books:ident) => {
        $index.iter().all(|book| {
            *$enabled_books
                .lock()
                .unwrap()
                .get(&book.product)
                .expect("Book does not exist")
        })
    };
}

macro_rules! get_book {
    ($books:expr, $message:ident) => {
        $books
            .get_mut(&$message.product)
            .expect("Book does not exist")
    };
}

pub struct AutoTrader {
    pub username: Username,
    pub password: String,
    pub books: HashMap<String, Book>,
    pub sequence: u32,
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
    const POSITION_LIMIT: i16 = 1000;
}

impl AutoTrader {
    pub fn new(username: Username, password: String) -> AutoTrader {
        AutoTrader {
            username,
            password,
            books: HashMap::new(),
            sequence: 0,
        }
    }

    pub async fn startup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (stream, response) =
            connect_async(url!("ws", AutoTrader::FEED_RECOVERY_PORT, "information"))
                .await
                .expect("Failed to connect to the websocket");
        println!("Server responded with headers: {:?}", response.headers());

        let messages: Vec<Message> = reqwest::get(url!(AutoTrader::FEED_RECOVERY_PORT, "recover"))
            .await?
            .json()
            .await?;
        for message in messages {
            self.sequence = message.sequence();
            self.parse_feed_message(message);
        }
        println!("Finished recovery books: {:#?}", self.books.keys());

        self.poll(stream.split().1).await?;
        Ok(())
    }

    /// Outbound decoder for feed
    fn parse_feed_message(&mut self, message: Message) {
        #[cfg(debug_assertions)]
        dbg!(&message);
        match message {
            Message::Future(future) => {
                assert_eq!(
                    future.expiry, future.halt_time,
                    "Expiry time should be the same as halt time",
                );
                self.books.insert(
                    future.product.clone(),
                    Book::new(future.product, future.station_id, future.expiry),
                );
            }
            Message::Added(added) => {
                get_book!(self.books, added).add_order(added, &self.username);
            }
            Message::Deleted(deleted) => {
                get_book!(self.books, deleted).remove_order(deleted, &self.username);
            }
            Message::Trade(trade) => {
                get_book!(self.books, trade).trade(trade, &self.username);
            }
            Message::Settlement(settlement) => {
                println!(
                    "Book {} settles at {:?}",
                    settlement.product, settlement.price,
                );
            }
            Message::Index(index) => {
                println!("Index definition: {index:#?}");
            }
            Message::TradingHalt(halt) => {
                self.books.remove(&halt.product);
            }
        }
    }

    pub async fn poll(
        &mut self,
        mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let enabled_books = Arc::new(Mutex::new(HashMap::new()));
        let orders_to_wait: Arc<Mutex<HashMap<String, String>>> =
            Arc::new(Mutex::new(HashMap::new()));
        while let Some(message) = stream.next().await {
            let message: Message = from_slice(&message?.into_data())?;
            let next_sequence = self.sequence + 1;
            #[allow(clippy::comparison_chain)]
            if message.sequence() == next_sequence {
                self.sequence = next_sequence;
                if let Message::Trade(ref trade) = message {
                    let mut orders_to_wait = orders_to_wait.lock().unwrap();
                    if let Some(order_id) = orders_to_wait.get(&trade.product) {
                        if trade.aggressor_order == order_id.clone() {
                            orders_to_wait.remove(&trade.product);
                        }
                    }
                }
                self.parse_feed_message(message);
            } else if message.sequence() > next_sequence {
                panic!(
                    "Expecting sequence number {} but got {}",
                    next_sequence,
                    message.sequence(),
                );
            }

            let mut indices: HashMap<String, [&Book; 4]> = HashMap::new();
            for book in self.books.values() {
                let position = book.position.position;
                let mut enabled_books = enabled_books.lock().unwrap();
                if !enabled_books.contains_key(&book.product) {
                    enabled_books.insert(
                        book.product.clone(),
                        if position > 0 {
                            position <= AutoTrader::POSITION_LIMIT
                        } else {
                            position >= -AutoTrader::POSITION_LIMIT
                        },
                    );
                }
                let entry = indices.entry(book.expiry.clone()).or_insert([&book; 4]);
                entry[book.station_id as usize] = book;
            }
            for index in indices.values() {
                if !index_enabled!(index, enabled_books, orders_to_wait) {
                    continue;
                }
                let orders = find_arbs(index, Price(500));
                for order in orders.iter() {
                    let position = self
                        .books
                        .get(&order.product)
                        .expect("Book does not exist")
                        .position
                        .position;
                    if position > 0 && position + order.volume > AutoTrader::POSITION_LIMIT
                        || position < 0 && position - order.volume < -AutoTrader::POSITION_LIMIT
                    {
                        // Disable the books that are about to go over position limit
                        if let Some(enable) = enabled_books.lock().unwrap().get_mut(&order.product)
                        {
                            *enable = false;
                        }
                    }
                }
                if index_enabled!(index, enabled_books) {
                    for order in orders {
                        // Disable the books where an order is about to be sent
                        enabled_books
                            .lock()
                            .unwrap()
                            .entry(order.product.clone())
                            .and_modify(|enabled| *enabled = false);
                        let username = self.username.clone();
                        let password = self.password.clone();
                        let orders_to_wait = orders_to_wait.clone();
                        let enabled_books = enabled_books.clone();
                        spawn(async move {
                            let result = send_order!(
                                to_string(&username)
                                    .expect("Failed to convert username to string")
                                    .trim_matches('"'),
                                password,
                                order
                            );
                            match result {
                                Ok(response) => {
                                    let response = response
                                        .text()
                                        .await
                                        .expect("Failed to parse add order response");
                                    match from_str::<OrderAddedMessage>(&response) {
                                        Ok(json) => {
                                            assert_eq!(
                                                json.resting, 0,
                                                "For IOC orders there should be no resting volume",
                                            );
                                            if json.filled > 0 {
                                                // Insert the book ID associated with the order ID if an order has been filled
                                                orders_to_wait
                                                    .lock()
                                                    .unwrap()
                                                    .insert(order.product.clone(), json.id.clone());
                                            }
                                            #[cfg(debug_assertions)]
                                            dbg!(json);
                                        }
                                        Err(err) => {
                                            dbg!(response, err);
                                        }
                                    }
                                }
                                Err(err) => {
                                    dbg!(err);
                                }
                            }
                            // Reenable book after receiving a response
                            enabled_books
                                .lock()
                                .unwrap()
                                .entry(order.product.clone())
                                .and_modify(|enabled| *enabled = true);
                        });
                    }
                }
            }
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
        types::{Price, Volume},
    };
    use serde_json::{from_value, json};
    use std::collections::BTreeMap;

    static PRODUCT: &str = "F_SOP_APP0104T0950";
    static EXPIRY: &str = "2024-01-04 09:50+1100";

    macro_rules! parse_json {
        ($trader:ident, $json:tt) => {
            $trader.parse_feed_message(
                from_value(json!($json)).expect("Failed to parse feed message"),
            );
        };
    }

    #[test]
    fn test_recovery() {
        let mut trader = AutoTrader::new(Username::KLiang, String::new());
        assert_eq!(trader.books, HashMap::new());

        parse_json!(trader, {
            "type": "FUTURE",
            "product": PRODUCT,
            "stationId": 66212,
            "stationName": "SYDNEY OLYMPIC PARK AWS (ARCHERY CENTRE)",
            "expiry": EXPIRY,
            "haltTime": EXPIRY,
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

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 18.98,
            "volume": 65,
            "buyer": "kliang",
            "seller": "prao",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "1",
            "passiveOrderRemaining": 30,
            "aggressorOrder": "2",
            "sequence": 2
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 65,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 18.98,
            "volume": 30,
            "buyer": "kliang",
            "seller": "prao",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "1",
            "passiveOrderRemaining": 0,
            "aggressorOrder": "3",
            "sequence": 3
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 95,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 28.90,
            "volume": 2,
            "buyer": "kliang",
            "seller": "kliang",
            "tradeType": "SELL_AGGRESSOR",
            "passiveOrder": "4",
            "passiveOrderRemaining": 3,
            "aggressorOrder": "5",
            "sequence": 4
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 95,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": PRODUCT,
            "id": "6",
            "side": "SELL",
            "price": 17.16,
            "filled": 0,
            "resting": 75,
            "owner": "kliang",
            "sequence": 5
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1716), Volume(75))]),
                    orders: HashMap::from([(
                        String::from("6"),
                        Order {
                            owner: Username::KLiang,
                            price: Price(1716),
                            volume: Volume(75),
                        },
                    )]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(75),
                        position: 95,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 17.16,
            "volume": 10,
            "buyer": "ssrzich",
            "seller": "kliang",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "6",
            "passiveOrderRemaining": 65,
            "aggressorOrder": "7",
            "sequence": 6
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1716), Volume(65))]),
                    orders: HashMap::from([(
                        String::from("6"),
                        Order {
                            owner: Username::KLiang,
                            price: Price(1716),
                            volume: Volume(65),
                        },
                    )]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(65),
                        position: 85,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 16.02,
            "volume": 23,
            "buyer": "kliang",
            "seller": "prao",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "8",
            "passiveOrderRemaining": 72,
            "aggressorOrder": "9",
            "sequence": 7
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1716), Volume(65))]),
                    orders: HashMap::from([(
                        String::from("6"),
                        Order {
                            owner: Username::KLiang,
                            price: Price(1716),
                            volume: Volume(65),
                        }
                    )]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(65),
                        position: 108,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 17.16,
            "volume": 10,
            "buyer": "ssrzich",
            "seller": "kliang",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "6",
            "passiveOrderRemaining": 55,
            "aggressorOrder": "10",
            "sequence": 8
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::from([(Price(1716), Volume(55))]),
                    orders: HashMap::from([(
                        String::from("6"),
                        Order {
                            owner: Username::KLiang,
                            price: Price(1716),
                            volume: Volume(55),
                        }
                    )]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(55),
                        position: 98,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );
    }

    #[test]
    fn test_feed() {
        let mut trader = AutoTrader::new(Username::KLiang, String::new());
        assert_eq!(trader.books, HashMap::new());

        parse_json!(trader, {
            "type": "FUTURE",
            "product": PRODUCT,
            "stationId": 66212,
            "stationName": "SYDNEY OLYMPIC PARK AWS (ARCHERY CENTRE)",
            "expiry": EXPIRY,
            "haltTime": EXPIRY,
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

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
                .expect("Book does not exist")
                .bbo(),
            (None, None),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                    orders: HashMap::new(),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": PRODUCT,
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
                .get(PRODUCT)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2431),
                    volume: Volume(20),
                }),
                None,
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": PRODUCT,
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
            "product": PRODUCT,
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
            "product": PRODUCT,
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
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "DELETED",
            "product": PRODUCT,
            "id": "1",
            "side": "BUY",
            "sequence": 6
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": PRODUCT,
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
            "product": PRODUCT,
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
            "product": PRODUCT,
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
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
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
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        // Wash trade
        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
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

        // Leftover volume on orderbook
        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 28.90,
            "volume": 2,
            "buyer": "kliang",
            "seller": "kliang",
            "tradeType": "SELL_AGGRESSOR",
            "passiveOrder": "6",
            "passiveOrderRemaining": 3,
            "aggressorOrder": "9",
            "sequence": 12
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        // Add to the same level as part of the best ask
        parse_json!(trader, {
            "type": "ADDED",
            "product": PRODUCT,
            "id": "10",
            "side": "SELL",
            "price": 31.01,
            "filled": 0,
            "resting": 2,
            "owner": "kliang",
            "sequence": 13
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2890),
                    volume: Volume(3),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(52),
                }),
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(15)), (Price(2890), Volume(3))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(52))]),
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
                        (
                            String::from("10"),
                            Order {
                                price: Price(3101),
                                owner: Username::KLiang,
                                volume: Volume(2),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(3),
                        ask_exposure: Volume(2),
                        position: -1,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 31.01,
            "volume": 1,
            "buyer": "cluo",
            "seller": "kliang",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "10",
            "passiveOrderRemaining": 1,
            "aggressorOrder": "11",
            "sequence": 14
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2890),
                    volume: Volume(3),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(51),
                }),
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(15)), (Price(2890), Volume(3))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(51))]),
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
                        (
                            String::from("10"),
                            Order {
                                price: Price(3101),
                                owner: Username::KLiang,
                                volume: Volume(1),
                            },
                        ),
                    ]),
                    position: Position {
                        bid_exposure: Volume(3),
                        ask_exposure: Volume(1),
                        position: -2,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 31.01,
            "volume": 1,
            "buyer": "slee2",
            "seller": "kliang",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "10",
            "passiveOrderRemaining": 0,
            "aggressorOrder": "12",
            "sequence": 15
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
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
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
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
                        position: -3,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADE",
            "product": PRODUCT,
            "price": 31.01,
            "volume": 2,
            "buyer": "slee2",
            "seller": "rcurby",
            "tradeType": "BUY_AGGRESSOR",
            "passiveOrder": "4",
            "passiveOrderRemaining": 48,
            "aggressorOrder": "12",
            "sequence": 16
        });

        assert_eq!(
            trader
                .books
                .get(PRODUCT)
                .expect("Book does not exist")
                .bbo(),
            (
                Some(PriceLevel {
                    price: Price(2890),
                    volume: Volume(3),
                }),
                Some(PriceLevel {
                    price: Price(3101),
                    volume: Volume(48),
                }),
            ),
        );
        assert_eq!(
            trader.books,
            HashMap::from([(
                PRODUCT.to_string(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(15)), (Price(2890), Volume(3))]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(48))]),
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
                                volume: Volume(48),
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
                        position: -3,
                    },
                    product: PRODUCT.to_string(),
                    station_id: Station::SydOlympicPark,
                    expiry: EXPIRY.to_string(),
                },
            )]),
        );

        parse_json!(trader, {
            "type": "TRADING_HALT",
            "product": PRODUCT,
            "sequence": 17
        });

        parse_json!(trader, {
            "type": "SETTLEMENT",
            "product": PRODUCT,
            "stationName": "SYDNEY OLYMPIC PARK AWS (ARCHERY CENTRE)",
            "expiry": EXPIRY,
            "price": 26.05,
            "sequence": 18
        });

        assert_eq!(trader.books, HashMap::new());
    }
}
