use serde::{Deserialize, Deserializer};

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
    product: String,
    station_id: StationId,
    station_name: String,
    expiry: String,
    halt_time: String,
    unit: String,
    strike: f32,
    aggressive_fee: f32,
    passive_fee: f32,
    announcement_fee: f32,
    incentive_rebate_per_unit: f32,
    max_incentive_rebate: f32,
    broker_fee: f32,
    timestamp: u64,
    sequence: u32,
}

#[derive(Deserialize)]
pub struct TradeRecovery {}

#[derive(Deserialize)]
pub struct AddedRecovery {}

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
