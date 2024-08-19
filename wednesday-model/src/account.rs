use crate::position::NormalizedPosition;

// Define a struct for a normalized crypto trading account
pub struct NormalizedCryptoAccount {
    pub account_id: String,
    pub balance: f64,
    pub positions: Vec<NormalizedPosition>,
    pub total_unrealized_pnl: f64,
    pub total_realized_pnl: f64,
}

impl NormalizedCryptoAccount {
    // Constructor for creating a new crypto account
    pub fn new(account_id: String, balance: f64) -> Self {
        Self {
            account_id,
            balance,
            positions: Vec::new(),
            total_unrealized_pnl: 0.0,
            total_realized_pnl: 0.0,
        }
    }

    // Method to add a new position to the account
    pub fn add_position(&mut self, position: NormalizedPosition) {
        self.total_unrealized_pnl += position.unrealized_pnl;
        self.positions.push(position);
    }

    // Method to update the price of a position
    pub fn update_position_price(&mut self, symbol: &str, new_price: f64) {
        for position in &mut self.positions {
            if position.symbol == symbol {
                self.total_unrealized_pnl -= position.unrealized_pnl;
                position.update_price(new_price);
                self.total_unrealized_pnl += position.unrealized_pnl;
            }
        }
    }

    // Method to realize PnL for a position
    pub fn realize_position_pnl(&mut self, symbol: &str, quantity_sold: f64, sale_price: f64) {
        for position in &mut self.positions {
            if position.symbol == symbol {
                self.total_unrealized_pnl -= position.unrealized_pnl;
                position.realize_pnl(quantity_sold, sale_price);
                self.total_realized_pnl += (sale_price - position.average_price) * quantity_sold;
                self.total_unrealized_pnl += position.unrealized_pnl;
            }
        }
    }
}
