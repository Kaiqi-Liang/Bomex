use reqwest::Error;
use observations::AutoTrader;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let response: Vec<Observation> = reqwest::get("http://sytev070:8090/current")
        .await?
        .json()
        .await?;
    println!("{response:#?}");
    let trader = AutoTrader {
        username: String::from("kliang"),
        password: String::from("de7d8b078d63d5d9ad4e9df2f542eca6"),
    };
    Ok(())
}
