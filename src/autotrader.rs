use reqwest::Error;
use serde::{Deserialize, Deserializer};
use crate::book::Book;
use crate::username::Username;

trait ConstantPorts {
    const OBSERVATION_PORT: u16;
    const EXECUTION_PORT: u16;
    const FEED_RECOVERY_PORT: u16;
}

pub struct AutoTrader {
    pub username: Username,
    pub password: String,
    pub host: String,
}

macro_rules! url {
    ($auto_trader:expr, $port:expr, $endpoint:expr) => {
        format!("http://{}:{}/{}", $auto_trader.host, $port, $endpoint)
    };
}

impl ConstantPorts for AutoTrader {
    const OBSERVATION_PORT: u16 = 8090;

    const EXECUTION_PORT: u16 = 9050;

    const FEED_RECOVERY_PORT: u16 = 9000;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    station: Station,
    time: String,
    air_temperature: f64,
    apparent_temperature: f64,
    barometric_pressure: f64,
    relative_humidity: u32,
    mystery: f64,
    wind_speed: u32,
    wind_direction: u32,
}

#[derive(Debug, Deserialize)]
enum Station {
    #[serde(rename = "066037")]
    SydAirport,
    #[serde(rename = "066212")]
    SydOlympicPark,
    #[serde(rename = "070351")]
    CanberraAirport,
    #[serde(rename = "058216")]
    CapeByron,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StationId {
    SydAirport = 66037,
    SydOlympicPark = 66212,
    CanberraAirport = 70351,
    CapeByron = 58216,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "type")]
enum Recovery {
    Future(FutureRecovery),
    Trade(TradeRecovery),
    Added(AddedRecovery),
    Index(IndexRecovery),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FutureRecovery {
    product: String,
    station_id: Station,
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

#[derive(Debug, Deserialize)]
struct TradeRecovery {}

#[derive(Debug, Deserialize)]
struct AddedRecovery {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexRecovery {
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

impl AutoTrader {
    pub async fn startup(&self) -> Result<(), Error> {
        self.recover().await
    }

    async fn recover(&self) -> Result<(), Error> {
        let response: Vec<Recovery> =
            reqwest::get(url!(self, AutoTrader::FEED_RECOVERY_PORT, "recover"))
                .await?
                .json()
                .await?;
        println!("{response:#?}");
		return Ok(());
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

    pub async fn refresh_latest_observations(&self) -> Result<(), Error> {
        let response: Vec<Observation> =
            reqwest::get(url!(self, AutoTrader::OBSERVATION_PORT, "current"))
                .await?
                .json()
                .await?;
        println!("{response:#?}");
        Ok(())
    }
}
