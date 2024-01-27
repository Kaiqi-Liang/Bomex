use crate::order::{AddMessage, BulkDeleteMessage, DeleteMessage, Message, MessageType, OrderType};
use crate::orderbook::{Book, Side};
use crate::recovery::Recovery;
use crate::username::Username;
use crate::{observations::Observation, order::Order};
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
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

trait ConstantPorts {
    const OBSERVATION_PORT: u16;
    const EXECUTION_PORT: u16;
    const FEED_RECOVERY_PORT: u16;
    const HOSTNAME: &'static str;
}

pub struct AutoTrader {
    username: Username,
    password: String,
    books: HashMap<String, Book>,
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

    pub async fn startup(&self) -> Result<(), reqwest::Error> {
        self.recover().await
    }

    async fn recover(&self) -> Result<(), reqwest::Error> {
        let response: Vec<Recovery> = reqwest::get(url!(AutoTrader::FEED_RECOVERY_PORT, "recover"))
            .await?
            .json()
            .await?;
        for message in response {
            match message {
                Recovery::Future(_) => todo!(),
                Recovery::Trade(_) => todo!(),
                Recovery::Added(_) => todo!(),
                Recovery::Index(_) => todo!(),
            }
        }
        Ok(())
    }

    pub async fn poll(&self) -> Result<(), tungstenite::Error> {
        let (stream, _) =
            connect_async(url!("ws", AutoTrader::FEED_RECOVERY_PORT, "information")).await?;
        let (_, read) = stream.split();
        let _ = read.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout().write_all(&data).await.unwrap();
        });
        Ok(())
    }

    pub async fn place_order(
        &self,
        product: &str,
        price: f64,
        side: Side,
        volume: u32,
        order_type: OrderType,
    ) -> Result<(), reqwest::Error> {
        send_order!(&Order {
            username: &self.username,
            password: &self.password,
            message: Message::Add(AddMessage {
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
            message: Message::Delete(DeleteMessage {
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
            message: Message::BulkDelete(BulkDeleteMessage {
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
        println!("{response:#?}");
        Ok(())
    }
}
