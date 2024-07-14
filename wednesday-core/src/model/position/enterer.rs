use uuid::Uuid;

use crate::model::{fee::Fees, fill_event::FillEvent, portfolio_error::PortfolioError};

use super::{determine_position_id, Position, PositionMeta};

/// Enters a new [`Position`].
pub trait PositionEnterer {
    /// Returns a new [`Position`], given an input [`FillEvent`] & an associated engine_id.
    fn enter(engine_id: Uuid, fill: &FillEvent) -> Result<Position, PortfolioError>;
}


impl PositionEnterer for Position {
    fn enter(engine_id: Uuid, fill: &FillEvent) -> Result<Position, PortfolioError> {
        // Initialise Position Metadata
        let metadata = PositionMeta {
            enter_timestamp: fill.market_meta.timestamp,
            update_timestamp: fill.timestamp,
            exit_balance: None,
        };

        // Enter fees
        let enter_fees_total = fill.fees.calculate_total_fees();

        // Enter price
        let enter_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Unreal profit & loss
        let unrealised_profit_loss = -enter_fees_total * 2.0;

        Ok(Position {
            position_id: determine_position_id(engine_id, &fill.exchange, &fill.instrument),
            exchange: fill.exchange.clone(),
            instrument: fill.instrument.clone(),
            meta: metadata,
            side: Position::parse_entry_side(fill)?,
            quantity: fill.quantity,
            enter_fees: fill.fees,
            enter_fees_total,
            enter_avg_price_gross,
            enter_value_gross: fill.fill_value_gross,
            exit_fees: Fees::default(),
            exit_fees_total: 0.0,
            exit_avg_price_gross: 0.0,
            exit_value_gross: 0.0,
            current_symbol_price: enter_avg_price_gross,
            current_value_gross: fill.fill_value_gross,
            unrealised_profit_loss,
            realised_profit_loss: 0.0,
        })
    }
}

