use serde::{Deserialize, Deserializer};

use crate::{orderbook::{Price, Side, Volume}, username::Username};

#[derive(Deserialize)]
#[serde(untagged)]
enum StationId {
    SydAirport = 66037,
    SydOlympicPark = 66212,
    CanberraAirport = 70351,
    CapeByron = 58216,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "type")]
pub enum Recovery {
    Future(FutureRecovery),
    Trade(TradeRecovery),
    Added(AddedRecovery),
    Index(IndexRecovery),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FutureRecovery {
    pub product: String,
    pub station_id: StationId,
    pub station_name: String,
    pub expiry: String,
    pub halt_time: String,
    pub unit: String,
    pub strike: Price,
    pub aggressive_fee: Price,
    pub passive_fee: Price,
    pub announcement_fee: Price,
    pub incentive_rebate_per_unit: Price,
    pub max_incentive_rebate: Price,
    pub broker_fee: Price,
    pub timestamp: u64,
    pub sequence: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRecovery {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedRecovery {
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
pub struct IndexRecovery {
    index_id: u32,
    index_name: String,
    #[serde(deserialize_with = "deserialize_station_ids")]
    station_ids: Vec<StationId>,
    timestamp: u64,
    sequence: u32,
}

fn deserialize_station_ids<'de, D>(deserializer: D) -> Result<Vec<StationId>, D::Error>
where
    D: Deserializer<'de>,
{
    let ids: Vec<u32> = Deserialize::deserialize(deserializer)?;
    let result: Vec<StationId> = ids.into_iter().map(StationId::from).collect();
    Ok(result)
}

impl From<u32> for StationId {
    fn from(id: u32) -> Self {
        match id {
            66037 => StationId::SydAirport,
            66212 => StationId::SydOlympicPark,
            70351 => StationId::CanberraAirport,
            58216 => StationId::CapeByron,
            _ => panic!("Unsupported ID value"),
        }
    }
}
