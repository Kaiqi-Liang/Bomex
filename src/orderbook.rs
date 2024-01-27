use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use crate::{
    feed::{AddedMessage, TradeMessage},
    pnl::PnL,
    username::Username,
};

pub type Volume = u16;
pub type Price = f64;

pub struct Book {
    // TODO: Use LinkedList<Arc<Order>>
    bids: BinaryHeap<Order>,
    asks: BinaryHeap<Order>,
    user_pnls: HashMap<Username, PnL>,
    is_active: bool,
    product_id: String,
}

macro_rules! get_pnl {
    ($pnls:expr, $user:expr) => {
        $pnls.get_mut($user).expect("User is not in the book")
    };
}

impl Book {
    pub fn new(product_id: String) -> Book {
        Book {
            bids: BinaryHeap::new(),
            asks: BinaryHeap::new(),
            user_pnls: HashMap::new(),
            is_active: true,
            product_id,
        }
    }

    pub fn add_order(&mut self, added: AddedMessage) {
        let (side, exposure) = if added.side == Side::Buy {
            (
                &mut self.bids,
                &mut get_pnl!(self.user_pnls, &added.owner).bid_exposure,
            )
        } else {
            (
                &mut self.asks,
                &mut get_pnl!(self.user_pnls, &added.owner).ask_exposure,
            )
        };
        *exposure += added.resting_volume;
        side.push(added.into());
    }

    pub fn trade(&mut self, trade: TradeMessage) {
        let pnl = trade.volume as f64 * trade.price;
        let volume = trade.volume as i16;

        let buyer = get_pnl!(self.user_pnls, &trade.buyer);
        buyer.position += volume;
        buyer.trade_pnl -= pnl;
        buyer.bought += trade.volume;

        if trade.buyer == trade.seller {
            buyer.washed += trade.volume;
        }

        let seller = get_pnl!(self.user_pnls, &trade.seller);
        seller.position -= volume;
        seller.trade_pnl += pnl;
        seller.sold += trade.volume;

        // TODO: keep track of exposure
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
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
