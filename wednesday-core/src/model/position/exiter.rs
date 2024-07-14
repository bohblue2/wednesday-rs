use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{
    balance::Balance,
    fee::{FeeAmount, Fees},
    fill_event::FillEvent,
    portfolio_error::PortfolioError,
};

use super::Position;

/// Exits an open [`Position`].
pub trait PositionExiter {
    /// Exits an open [`Position`], given the input Portfolio equity & the [`FillEvent`] returned
    /// from an Execution handler.
    fn exit(&mut self, balance: Balance, fill: &FillEvent) -> Result<PositionExit, PortfolioError>;
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionExit {
    /// Unique identifier for a [`Position`], generated from an exchange, symbol, and enter_time.
    pub position_id: String,

    /// [`FillEvent`] timestamp that triggered the exiting of this [`Position`].
    pub exit_time: DateTime<Utc>,

    /// Portfolio [`Balance`] calculated at the point of exiting a [`Position`].
    pub exit_balance: Balance,

    /// All fees types incurred from exiting a [`Position`], and their associated [`FeeAmount`].
    pub exit_fees: Fees,

    /// Total of exit_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub exit_fees_total: FeeAmount,

    /// Exit average price excluding the exit_fees_total.
    pub exit_avg_price_gross: f64,

    /// abs(Quantity) * exit_avg_price_gross.
    pub exit_value_gross: f64,

    /// Realised P&L after the [`Position`] has closed.
    pub realised_profit_loss: f64,
}

impl TryFrom<&mut Position> for PositionExit {
    type Error = PortfolioError;

    fn try_from(exited_position: &mut Position) -> Result<Self, Self::Error> {
        Ok(Self {
            position_id: exited_position.position_id.clone(),
            exit_time: exited_position.meta.update_timestamp,
            exit_balance: exited_position.meta.exit_balance.ok_or(PortfolioError::PositionExit)?,
            exit_fees: exited_position.exit_fees,
            exit_fees_total: exited_position.exit_fees_total,
            exit_avg_price_gross: exited_position.exit_avg_price_gross,
            exit_value_gross: exited_position.exit_value_gross,
            realised_profit_loss: exited_position.realised_profit_loss,
        })
    }
}

impl PositionExiter for Position {
    fn exit(&mut self, mut balance: Balance, fill: &FillEvent) -> Result<PositionExit, PortfolioError> {
        if fill.decision.is_entry() {
            return Err(PortfolioError::CannotExitPositionWithEntryFill);
        }

        // Exit fees
        self.exit_fees = fill.fees;
        self.exit_fees_total = fill.fees.calculate_total_fees();

        // Exit value & price
        self.exit_value_gross = fill.fill_value_gross;
        self.exit_avg_price_gross = Position::calculate_avg_price_gross(fill);

        // Result profit & loss
        self.realised_profit_loss = self.calculate_realised_profit_loss();
        self.unrealised_profit_loss = self.realised_profit_loss;

        // Metadata
        balance.total += self.realised_profit_loss;
        self.meta.update_timestamp = fill.timestamp;
        self.meta.exit_balance = Some(balance);

        PositionExit::try_from(self)
    }
}
