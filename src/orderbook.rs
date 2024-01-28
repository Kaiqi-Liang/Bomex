use crate::{
    feed::{AddedMessage, DeletedMessage, TradeMessage},
    types::{Price, Side, Volume},
    username::Username,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, PartialEq)]
pub struct Book {
    pub bids: BTreeMap<Price, Volume>,
    pub asks: BTreeMap<Price, Volume>,
    pub orders: HashMap<String, Order>,
    pub position: Position,
    pub is_active: bool,
}

#[derive(Debug, PartialEq)]
pub struct Order {
    pub owner: Username,
    pub price: Price,
    pub volume: Volume,
}

impl From<AddedMessage> for Order {
    fn from(added: AddedMessage) -> Self {
        Order {
            owner: added.owner,
            price: added.price,
            volume: added.resting,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub bid_exposure: Volume, // open bid exposure in the market
    pub ask_exposure: Volume, // open ask exposure in the market
    pub position: i16, // current traded position in the book. If the book is active, this is their 'current position'. If the book has settled, then this is was the user's position as settlement time.
}

macro_rules! get_side_and_exposure {
    ($self:ident, $side:expr) => {
        if $side == Side::Buy {
            (&mut $self.bids, &mut $self.position.bid_exposure)
        } else {
            (&mut $self.asks, &mut $self.position.ask_exposure)
        }
    };
}

impl Book {
    pub fn new() -> Book {
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
        }
    }

    pub fn add_order(&mut self, added: AddedMessage, username: &Username) {
        let (side, exposure) = get_side_and_exposure!(self, added.side);
        if added.owner == *username {
            *exposure += added.resting;
        }
        side.entry(added.price)
            .and_modify(|volume| *volume += added.resting)
            .or_insert(added.resting);
        self.orders.insert(added.id.clone(), added.into());
    }

    pub fn remove_order(&mut self, deleted: DeletedMessage, username: &Username) {
        let order = self
            .orders
            .remove(&deleted.id)
            .expect("Trying to delete an order with unknown ID");
        let (side, exposure) = get_side_and_exposure!(self, deleted.side);
        *side
            .get_mut(&order.price)
            .expect("Order does not exist in the orderbook") -= order.volume;
        if order.owner == *username {
            *exposure -= order.volume;
        }
    }

    pub fn trade(&mut self, trade: TradeMessage, username: &Username) {
        if trade.buyer == *username || trade.seller == *username {
            if trade.buyer == trade.seller {
                self.position.ask_exposure -= trade.volume;
                self.position.bid_exposure -= trade.volume;
            } else if trade.buyer == *username {
                self.position.position += trade.volume;
            } else if trade.seller == *username {
                self.position.position -= trade.volume;
            }
        }
    }
}
