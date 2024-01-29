use crate::{
    observations::Station,
    types::{Price, Side, Volume},
    username::Username,
};
use serde::{Deserialize, Deserializer};

pub trait HasSequence {
    fn sequence(&self) -> u32;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum Message {
    Future(FutureMessage),
    Added(AddedMessage),
    Deleted(DeletedMessage),
    Trade(TradeMessage),
    Settlement(SettlementMessage),
    Index(IndexMessage),
    TradingHalt(TradingHaltMessage),
}

impl HasSequence for Message {
    fn sequence(&self) -> u32 {
        match self {
            Message::Future(message) => message.sequence,
            Message::Added(message) => message.sequence,
            Message::Deleted(message) => message.sequence,
            Message::Trade(message) => message.sequence,
            Message::Settlement(message) => message.sequence,
            Message::Index(message) => message.sequence,
            Message::TradingHalt(message) => message.sequence,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FutureMessage {
    pub product: String,
    pub station_id: Station,
    pub station_name: String,
    pub expiry: String,
    pub halt_time: String,
    pub sequence: u32,
}

#[derive(Debug, Deserialize)]
pub struct AddedMessage {
    pub product: String,
    pub id: String,
    pub side: Side,
    pub price: Price,
    pub filled: Volume,
    pub resting: Volume,
    pub owner: Username,
    pub sequence: u32,
}

#[derive(Debug, Deserialize)]
pub struct DeletedMessage {
    pub product: String,
    pub id: String,
    pub side: Side,
    pub sequence: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMessage {
    pub product: String,
    pub price: Price,
    pub volume: Volume,
    pub buyer: Username,
    pub seller: Username,
    pub trade_type: TradeType,
    pub passive_order: String,
    pub passive_order_remaining: Volume,
    pub aggressor_order: String,
    pub sequence: u32,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeType {
    SellAggressor,
    BuyAggressor,
    BrokerTrade,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementMessage {
    pub product: String,
    pub station_name: String,
    #[allow(dead_code)]
    expiry: String,
    pub price: Price,
    pub sequence: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct IndexMessage {
    index_id: u32,
    index_name: String,
    #[serde(deserialize_with = "deserialize_station_ids")]
    station_ids: Vec<Station>,
    pub sequence: u32,
}

fn deserialize_station_ids<'de, D>(deserializer: D) -> Result<Vec<Station>, D::Error>
where
    D: Deserializer<'de>,
{
    let ids: Vec<u64> = Deserialize::deserialize(deserializer)?;
    Ok(ids.into_iter().map(Station::from).collect())
}

#[derive(Debug, Deserialize)]
pub struct TradingHaltMessage {
    pub product: String,
    pub sequence: u32,
}
