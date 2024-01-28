use crate::{
    observations::Observation,
    order::{AddMessage, BulkDeleteMessage, DeleteMessage, MessageType, Order, OrderType},
    orderbook::Book,
    types::{Price, Side, Volume},
    username::Username,
};
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite};

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

pub struct AutoTrader {
    username: Username,
    password: String,
    books: HashMap<String, Book>,
}

trait ConstantPorts {
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
    pub fn new(username: Username, password: String) -> AutoTrader {
        AutoTrader {
            username,
            password,
            books: HashMap::new(),
        }
    }

    pub async fn startup(&mut self) -> Result<(), reqwest::Error> {
        self.recover().await
    }

    async fn recover(&mut self) -> Result<(), reqwest::Error> {
        let messages: Vec<crate::feed::Message> =
            reqwest::get(url!(AutoTrader::FEED_RECOVERY_PORT, "recover"))
                .await?
                .json()
                .await?;
        for message in messages {
            self.parse_feed_message(message);
        }
        Ok(())
    }

    fn parse_feed_message(&mut self, message: crate::feed::Message) {
        match message {
            crate::feed::Message::Future(future) => {
                self.books.insert(future.product, Book::new());
            }
            crate::feed::Message::Added(added) => {
                get_book!(self.books, added).add_order(added, &self.username);
            }
            crate::feed::Message::Trade(trade) => {
                get_book!(self.books, trade).trade(trade, &self.username);
            }
            crate::feed::Message::Index(index) => todo!(),
        }
    }

    pub async fn poll(&mut self) -> Result<(), tungstenite::Error> {
        let (stream, _) =
            connect_async(url!("ws", AutoTrader::FEED_RECOVERY_PORT, "information")).await?;
        let (_, read) = stream.split();
        let _ = read.for_each(|message| async {
            let message: crate::feed::Message =
                serde_json::from_slice(&message.unwrap().into_data()).expect("Failed to parse feed message");
            // self.parse_feed_message(message);
        });
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
        send_order!(&Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::Add(AddMessage {
                message_type: MessageType::Add,
                product,
                price,
                side,
                volume,
                order_type
            }),
        });
        Ok(())
    }

    pub async fn cancel_order(&self, product: &str, id: &str) -> Result<(), reqwest::Error> {
        send_order!(&Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::Delete(DeleteMessage {
                message_type: MessageType::Delete,
                product,
                id
            })
        });
        Ok(())
    }

    pub async fn cancel_all_orders_in_book(&self, product: &str) -> Result<(), reqwest::Error> {
        send_order!(&Order {
            username: &self.username,
            password: &self.password,
            message: crate::order::Message::BulkDelete(BulkDeleteMessage {
                message_type: MessageType::BulkDelete,
                product
            })
        });
        Ok(())
    }

    pub async fn refresh_latest_observations(&self) -> Result<(), reqwest::Error> {
        let response: Vec<Observation> =
            reqwest::get(url!(AutoTrader::OBSERVATION_PORT, "current"))
                .await?
                .json()
                .await?;
        println!("{}", response[0].air_temperature);
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.poll().await?;
        for id in self.books.keys() {
            self.cancel_all_orders_in_book(id).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orderbook::Position;
    use serde_json::{from_value, json};
    use std::collections::BTreeMap;

    macro_rules! parse_json {
        ($trader:ident, $json:tt) => {
            $trader.parse_feed_message(from_value(json!($json)).expect("Failed to parse feed message"));
        };
    }

    #[test]
    fn test_parse_feed_message() {
        let mut trader = AutoTrader::new(
            Username::KLiang,
            String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
        );
        assert_eq!(trader.books, HashMap::new());

        let product = String::from("F_SOP_APP0104T0950");
        let order1 = String::from("02a70d7e-9178-46df-8f77-a996f2e78bd8");
        let order2 = String::from("defb5c1b-e580-4a8e-a668-3847f5ccad16");
        let order3 = String::from("678306ab-9771-4e53-9c61-ba3451847d74");
        let order4 = String::from("cbaf100a-b9c5-4a77-8499-3a6b176e157f");
        let order5 = String::from("af0261ad-cb51-44f6-842d-520afd2ec392");

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
            "id": order1,
            "side": "BUY",
            "price": 24.31,
            "filled": 0,
            "resting": 20,
            "owner": "cchuah",
            "sequence": 2
        });
        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(20))]),
                    asks: BTreeMap::new(),
                    orders: HashMap::from([(order1.clone(), Price(2431))]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    is_active: true,
                },
            )])
        );

        parse_json!(trader, {
            "type": "ADDED",
            "product": product,
            "id": order2,
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
            "id": order3,
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
            "id": order4,
            "side": "SELL",
            "price": 31.01,
            "filled": 0,
            "resting": 50,
            "owner": "rcurby",
            "sequence": 5
        });

        parse_json!(trader, {
            "type": "TRADE",
            "product": product,
            "price": 28.35,
            "volume": 5,
            "buyer": "fmavlono",
            "seller": "pshannon",
            "tradeType": "SELL_AGGRESSOR",
            "passiveOrder": order2,
            "passiveOrderRemaining": 0,
            "aggressorOrder": order5,
            "sequence": 6
        });

        assert_eq!(
            trader.books,
            HashMap::from([(
                product.clone(),
                Book {
                    bids: BTreeMap::from([(Price(2431), Volume(35)),]),
                    asks: BTreeMap::from([(Price(3329), Volume(20)), (Price(3101), Volume(50)),]),
                    orders: HashMap::from([
                        (order1.clone(), Price(2431)),
                        (order2.clone(), Price(2431)),
                        (order3.clone(), Price(3329)),
                        (order4.clone(), Price(3101)),
                    ]),
                    position: Position {
                        bid_exposure: Volume(0),
                        ask_exposure: Volume(0),
                        position: 0,
                    },
                    is_active: true,
                },
            )])
        );
    }
}
