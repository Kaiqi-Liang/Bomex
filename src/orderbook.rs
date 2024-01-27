use crate::{
    feed::{AddedMessage, TradeMessage},
    types::{Price, Side, Volume},
    username::Username,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

pub struct Book {
    bids: BTreeMap<Price, Volume>,
    asks: BTreeMap<Price, Volume>,
    orders: HashMap<String, Price>,
    position: Position,
    is_active: bool,
    product_id: String,
}

pub struct Position {
    pub bid_exposure: Volume, // open bid exposure in the market
    pub ask_exposure: Volume, // open ask exposure in the market
    pub position: i16, // current traded position in the book. If the book is active, this is their 'current position'. If the book has settled, then this is was the user's position as settlement time.
}

impl Book {
    pub fn new(product_id: String) -> Book {
        Book {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            position: Position {
                bid_exposure: Volume(0),
                ask_exposure: Volume(0),
                position: 0,
            },
            is_active: true,
            product_id,
        }
    }

    pub fn add_order(&mut self, added: AddedMessage, username: &Username) {
        let (side, exposure) = if added.side == Side::Buy {
            (&mut self.bids, &mut self.position.bid_exposure)
        } else {
            (&mut self.asks, &mut self.position.ask_exposure)
        };
        if added.owner == *username {
            *exposure += added.resting_volume;
        }
        let level = side
            .entry(added.price)
            .and_modify(|volume| *volume += added.resting_volume)
            .or_insert(added.resting_volume);
        self.orders.insert(added.order_id, added.price);
    }

    pub fn trade(&mut self, trade: TradeMessage, username: &Username) {
        if trade.buyer != *username && trade.seller != *username {
            return;
        } else if trade.buyer == trade.seller {
            self.position.ask_exposure -= trade.volume;
            self.position.bid_exposure -= trade.volume;
        } else if trade.buyer == *username {
            self.position.position += trade.volume;
        } else if trade.seller == *username {
            self.position.position -= trade.volume;
        }
    }
}

impl From<AddedMessage> for Order {
    fn from(added: AddedMessage) -> Self {
        Order {
            order_id: added.order_id,
            owner: added.owner,
            price: added.price,
            side: added.side,
            filled_volume: added.filled_volume,
            resting_volume: added.resting_volume,
        }
    }
}

struct Order {
    order_id: String,
    owner: Username,
    price: Price,
    side: Side,
    filled_volume: Volume,
    resting_volume: Volume,
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        assert_eq!(self.side, other.side);
        match self.side {
            Side::Buy => self.price.partial_cmp(&other.price),
            Side::Sell => other.price.partial_cmp(&self.price),
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
