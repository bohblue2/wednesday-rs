// Define a struct for a normalized trading position
pub struct NormalizedPosition {
    pub symbol: String,
    pub quantity: f64,
    pub average_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
}

impl NormalizedPosition {
    // Constructor for creating a new normalized position
    pub fn new(symbol: String, quantity: f64, average_price: f64, current_price: f64) -> Self {
        let unrealized_pnl = (current_price - average_price) * quantity;
        Self {
            symbol,
            quantity,
            average_price,
            current_price,
            unrealized_pnl,
            realized_pnl: 0.0,
        }
    }

    // Method to update the current price and recalculate unrealized PnL
    pub fn update_price(&mut self, new_price: f64) {
        self.current_price = new_price;
        self.unrealized_pnl = (self.current_price - self.average_price) * self.quantity;
    }

    // Method to realize PnL
    pub fn realize_pnl(&mut self, quantity_sold: f64, sale_price: f64) {
        let pnl = (sale_price - self.average_price) * quantity_sold;
        self.realized_pnl += pnl;
        self.quantity -= quantity_sold;
        self.unrealized_pnl = (self.current_price - self.average_price) * self.quantity;
    }
}
