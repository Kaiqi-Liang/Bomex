use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, SubAssign};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(u16);

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
