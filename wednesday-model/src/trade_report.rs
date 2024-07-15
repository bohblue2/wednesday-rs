// Define a struct for a normalized crypto trade report
pub struct NormalizedCryptoTradeReport {
    pub trade_id: String,
    pub account_id: String,
    pub symbol: String,
    pub trade_type: TradeType,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: u64,
    pub fees: f64,
    pub pnl: f64,
}

pub enum TradeType {
    Buy,
    Sell,
}

impl NormalizedCryptoTradeReport {
    // Constructor for creating a new trade report
    pub fn new(
        trade_id: String,
        account_id: String,
        symbol: String,
        trade_type: TradeType,
        quantity: f64,
        price: f64,
        timestamp: u64,
        fees: f64,
    ) -> Self {
        let pnl = match trade_type {
            TradeType::Buy => 0.0, // PnL is not realized on buy
            TradeType::Sell => (price - 0.0) * quantity - fees, // Simplified PnL calculation
        };
        Self {
            trade_id,
            account_id,
            symbol,
            trade_type,
            quantity,
            price,
            timestamp,
            fees,
            pnl,
        }
    }

    // Method to update the PnL after a trade
    pub fn update_pnl(&mut self, average_price: f64) {
        self.pnl = match self.trade_type {
            TradeType::Buy => 0.0, // PnL is not realized on buy
            TradeType::Sell => (self.price - average_price) * self.quantity - self.fees,
        };
    }
}
