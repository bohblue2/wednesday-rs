use crate::orderbook::Level;


pub fn mid_price(best_bid_price: f64, best_ask_price: f64) -> f64 {
    (best_bid_price + best_ask_price) / 2.0
}

pub fn volume_weighted_mid_price(best_bid: Level, best_ask: Level) -> f64 {
    ((best_bid.price * best_ask.amount) + (best_ask.price * best_bid.amount))
        / (best_bid.amount + best_ask.amount)
}