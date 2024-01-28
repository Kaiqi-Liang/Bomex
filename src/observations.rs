use crate::types::Station;
use serde::Deserialize;

#[derive(Deserialize)]
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
