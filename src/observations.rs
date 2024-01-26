use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
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
