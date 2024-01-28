use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{AddAssign, SubAssign};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub u16);

impl From<f64> for Price {
    fn from(value: f64) -> Self {
        Price((value * 100.0) as u16)
    }
}

impl Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let price = self.0 as f64 / 100.0;
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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Volume(pub u16);

impl AddAssign for Volume {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Volume {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl AddAssign<Volume> for i16 {
    fn add_assign(&mut self, rhs: Volume) {
        *self += rhs.0 as i16;
    }
}

impl SubAssign<Volume> for i16 {
    fn sub_assign(&mut self, rhs: Volume) {
        *self -= rhs.0 as i16;
    }
}
