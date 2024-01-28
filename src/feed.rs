use crate::{
    types::{Price, Side, Station, Volume},
    username::Username,
};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "type")]
pub enum Message {
    Future(FutureMessage),
    Added(AddedMessage),
    Trade(TradeMessage),
    Index(IndexMessage),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FutureMessage {
    pub product: String,
    pub station_id: Station,
    pub station_name: String,
    pub expiry: String,
    pub halt_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedMessage {
    pub product: String,
    pub order_id: String,
    pub side: Side,
    pub price: Price,
    pub filled_volume: Volume,
    pub resting_volume: Volume,
    pub owner: Username,
}

#[derive(Deserialize)]
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
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TradeType {
    SellAggressor,
    BuyAggressor,
    BrokerTrade,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMessage {
    index_id: u32,
    index_name: String,
    #[serde(deserialize_with = "deserialize_station_ids")]
    station_ids: Vec<Station>,
}

fn deserialize_station_ids<'de, D>(deserializer: D) -> Result<Vec<Station>, D::Error>
where
    D: Deserializer<'de>,
{
    let ids: Vec<u64> = Deserialize::deserialize(deserializer)?;
    let result: Vec<Station> = ids.into_iter().map(Station::from).collect();
    Ok(result)
}
