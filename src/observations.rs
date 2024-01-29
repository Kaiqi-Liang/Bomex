use crate::{
    autotrader::{AutoTrader, ConstantPorts},
    url,
};
use serde::{de::Error, Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub station: Station,
    pub time: String,
    pub air_temperature: f64,
    pub apparent_temperature: f64,
    pub barometric_pressure: f64,
    pub relative_humidity: u32,
    pub mystery: f64,
    pub wind_speed: u32,
    pub wind_direction: u32,
}

pub struct Observations {
    pub observations: HashMap<Station, Vec<Observation>>,
}

impl Observations {
    pub async fn refresh_latest_observations(&mut self) -> Result<(), reqwest::Error> {
        let response: Vec<Observation> =
            reqwest::get(url!(AutoTrader::OBSERVATION_PORT, "current"))
                .await?
                .json()
                .await?;
        for observation in response {
            self.observations
                .entry(observation.station.clone())
                .and_modify(|observations| observations.push(observation));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Station {
    SydAirport,
    SydOlympicPark,
    CanberraAirport,
    CapeByron,
}

impl From<u64> for Station {
    fn from(id: u64) -> Self {
        match id {
            66037 => Station::SydAirport,
            66212 => Station::SydOlympicPark,
            70351 => Station::CanberraAirport,
            58216 => Station::CapeByron,
            _ => panic!("Unknown Station ID"),
        }
    }
}

impl<'de> Deserialize<'de> for Station {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let station = Deserialize::deserialize(deserializer)?;
        match station {
            Value::Number(station) => Ok(station
                .as_u64()
                .ok_or(D::Error::custom("Invalid station ID format"))?
                .into()),
            Value::String(station) => Ok(station.parse::<u64>().map_err(D::Error::custom)?.into()),
            _ => Err(D::Error::custom("Invalid station ID format")),
        }
    }
}
