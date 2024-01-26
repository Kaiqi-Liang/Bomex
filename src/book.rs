use crate::order::Order;
use std::collections::BinaryHeap;

pub struct Book {
    bids: BinaryHeap<Order>,
    asks: BinaryHeap<Order>,
}
