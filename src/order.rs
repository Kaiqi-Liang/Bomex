use crate::username::Username;
use std::cmp::Ordering;

#[derive(Debug, PartialEq)]
enum Side {
    BUY,
    SELL,
}

#[derive(Debug)]
pub struct Order {
    order_id: String,
    owner: Username,
    price: f64,
    side: Side,
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        assert_eq!(self.side, other.side);
        match self.side {
            Side::BUY => self.price.partial_cmp(&other.price),
            Side::SELL => other.price.partial_cmp(&self.price),
        }
        .unwrap()
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl Eq for Order {}
