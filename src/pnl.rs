use crate::{
    orderbook::{Price, Volume},
    username::Username,
};

pub struct PnL {
    pub owner: Username,       // username
    pub bid_exposure: Volume,  // open bid exposure in the market
    pub ask_exposure: Volume,  // open ask exposure in the market
    pub position: i16, // current traded position in the book. If the book is active, this is their 'current position'. If the book has settled, then this is was the user's position as settlement time.
    pub trade_pnl: Price, // how much money the user has earned/spent by buying and selling, but not settling
    pub settlement_pnl: Price, // how much money the user has earned/lost due to settlements. Will be zero for any book that is still active
    pub fees: Price, // how much money the user has payed in transaction fees. tracked separately to trade PnL in order to make it clear how fees are affecting things. May be negative due to rebates.
    pub penalties: Price, // how much money the user has been penalised for non-compliant behaviour for this book. Should hopefully just be zero!
    pub bought: Volume,   // how much volume this user has bought
    pub sold: Volume,     // how much volume this user has sold
    pub hit: Volume,      // how much volume this user has traded aggressively
    pub quoted: Volume,   // how much volume this user has traded passively
    pub washed: Volume,   // how much volume this user has wash traded
    pub settled: bool,    // whether or not the product in question has actually settled
}
