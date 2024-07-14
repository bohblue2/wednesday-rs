use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wednesday_model::events::{DataKind, MarketEvent};

use super::{Position, PositionId};

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionUpdate {
    pub position_id: PositionId,
    pub update_timestamp: DateTime<Utc>,
    pub current_symbol_price: f64,
    pub current_value_gross: f64,
    pub unrealised_profit_loss: f64,
}

impl From<&mut Position> for PositionUpdate {
    fn from(updated_position: &mut Position) -> Self {
        Self {
            position_id: updated_position.position_id.clone(),
            update_timestamp: updated_position.meta.update_timestamp,
            current_symbol_price: updated_position.current_symbol_price,
            current_value_gross: updated_position.current_value_gross,
            unrealised_profit_loss: updated_position.unrealised_profit_loss,
        }
    }
}

/// Updates an open [`Position`].
pub trait PositionUpdater {
    /// Updates an open [`Position`] using the latest input [`MarketEvent`], returning a
    /// [`PositionUpdate`] that communicates the open [`Position`]'s change in state.
    fn update(&mut self, market: &MarketEvent<DataKind>) -> Option<PositionUpdate>;
}

impl PositionUpdater for Position {
    fn update(&mut self, market: &MarketEvent<DataKind>) -> Option<PositionUpdate> {
        // Determine close from MarketEvent
        let close = match &market.kind {
            DataKind::PublicTrade(trade) => trade.price,
            DataKind::OrderBookL1(book_l1) => book_l1.volume_weighed_mid_price(),
            DataKind::Bar(bar) => bar.close,
            _ => return None,
        };

        self.meta.update_timestamp = market.exchange_ts;

        self.current_symbol_price = close;

        // Market value gross
        self.current_value_gross = close * self.quantity.abs();

        // Unreal profit & loss
        self.unrealised_profit_loss = self.calculate_unrealised_profit_loss();

        // Return a PositionUpdate event that communicates the change in state
        Some(PositionUpdate::from(self))
    }
}
