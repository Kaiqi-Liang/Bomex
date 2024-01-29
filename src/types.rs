use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u16);

impl Debug for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_underlying!(self) as f64 / 100.0)
    }
}

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
