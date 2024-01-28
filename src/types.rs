use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::ops::{AddAssign, Sub, SubAssign};

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

impl From<f64> for Price {
    fn from(value: f64) -> Self {
        let value = value * 100.0;
        assert!((value) % 1.0 == 0.0, "Value must have 2 decimal places");
        Price((value) as u16)
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

pub enum Station {
    SydAirport = 66037,
    SydOlympicPark = 66212,
    CanberraAirport = 70351,
    CapeByron = 58216,
}

impl<'de> Deserialize<'de> for Station {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let station: Value = Deserialize::deserialize(deserializer)?;
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
