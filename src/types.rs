use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::ops::{Add, AddAssign, Sub, SubAssign};

macro_rules! to_underlying {
    ($strong:expr) => {
        $strong.0
    };
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u16);

impl Sub<u16> for Price {
    type Output = Price;
    fn sub(self, rhs: u16) -> Self::Output {
        Price(to_underlying!(self) - rhs)
    }
}

impl Add<u16> for Price {
    type Output = Price;
    fn add(self, rhs: u16) -> Self::Output {
        Price(to_underlying!(self) + rhs)
    }
}

impl From<f64> for Price {
    fn from(price: f64) -> Self {
        Price((price * 100.0) as u16)
    }
}

impl Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let price = to_underlying!(self) as f64 / 100.0;
        price.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Price {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let price: f64 = Deserialize::deserialize(deserializer)?;
        Ok(price.into())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Volume(pub u16);

impl PartialEq for Volume {
    fn eq(&self, other: &Volume) -> bool {
        to_underlying!(self) == to_underlying!(other)
    }
}

impl PartialEq<u16> for Volume {
    fn eq(&self, other: &u16) -> bool {
        to_underlying!(self) == *other
    }
}

impl Sub for Volume {
    type Output = Volume;
    fn sub(self, rhs: Self) -> Self::Output {
        Volume(to_underlying!(self) - to_underlying!(rhs))
    }
}

impl AddAssign for Volume {
    fn add_assign(&mut self, rhs: Self) {
        to_underlying!(self) += to_underlying!(rhs);
    }
}

impl SubAssign for Volume {
    fn sub_assign(&mut self, rhs: Self) {
        to_underlying!(self) -= to_underlying!(rhs);
    }
}

impl AddAssign<Volume> for i16 {
    fn add_assign(&mut self, rhs: Volume) {
        *self += to_underlying!(rhs) as i16;
    }
}

impl SubAssign<Volume> for i16 {
    fn sub_assign(&mut self, rhs: Volume) {
        *self -= to_underlying!(rhs) as i16;
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
