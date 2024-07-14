use serde::{Deserialize, Serialize};

pub type FeeAmount = f64;

#[derive(Copy, Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub struct Fees {
    pub exchange: FeeAmount,
    pub slippage: FeeAmount,
}

impl Fees {
    pub fn calculate_total_fees(&self) -> FeeAmount {
        self.exchange + self.slippage
    }
}

impl Default for Fees {
    fn default() -> Self {
        Self {
            exchange: 0.0,
            slippage: 0.0,
        }
    }
}
