use crate::{
    autotrader::{AutoTrader, ConstantPorts},
    url,
};
use serde::{de::Error, Deserialize, Deserializer};
use serde_json::Value;
use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Debug, Deserialize)]
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

impl Ord for Observation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for Observation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Observation {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for Observation {}

#[allow(dead_code)]
pub fn poll_observations() {
    let observations = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let observations_clone = observations.clone();
    tokio::spawn(async move {
        loop {
            let result = get_latest_observations(observations_clone.clone()).await;
            if let Err(err) = result {
                dbg!(err);
            }
        }
    });
    println!("{observations:#?}");
}

async fn get_latest_observations(
    observations: Arc<Mutex<HashMap<Station, BTreeSet<Observation>>>>,
) -> Result<(), reqwest::Error> {
    let response: Vec<Observation> = reqwest::get(url!(AutoTrader::OBSERVATION_PORT, "current"))
        .await?
        .json()
        .await?;
    for observation in response {
        let mut observations = observations.lock().unwrap();
        let existing_observations = observations.entry(observation.station).or_default();
        if !existing_observations.contains(&observation) {
            existing_observations.insert(observation);
        }
    }
    Ok(())
}

#[derive(Default, Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Station {
    SydAirport = 0,
    SydOlympicPark = 1,
    CanberraAirport = 2,
    Index = 3,
    #[default]
    CapeByron,
}

impl From<u64> for Station {
    fn from(id: u64) -> Self {
        match id {
            66037 => Station::SydAirport,
            66212 => Station::SydOlympicPark,
            70351 => Station::CanberraAirport,
            58216 => Station::CapeByron,
            1 => Station::Index,
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
