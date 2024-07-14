pub mod builder;
pub mod enterer;
pub mod exiter;
pub mod updater;

use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wednesday_model::{identifiers::Exchange, instruments::Instrument};

use self::builder::PositionBuilder;

use super::{
    balance::Balance,
    decision::Decision,
    fee::{FeeAmount, Fees},
    fill_event::FillEvent,
    portfolio_error::PortfolioError,
};

pub type PositionId = String;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PositionMeta {
    pub enter_timestamp: DateTime<Utc>,
    pub update_timestamp: DateTime<Utc>,
    // Porfolio [`Balance`] calculated at the time of exiting the [`Position`]
    pub exit_balance: Option<Balance>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum PositionSide {
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}

impl Display for PositionSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PositionSide::Buy => "Buy",
                PositionSide::Sell => "Sell",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub position_id: PositionId,
    pub meta: PositionMeta,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub side: PositionSide,
    pub quantity: f64,
    /// All fees types incurred from entering a [`Position`], and their associated [`FeeAmount`].
    pub enter_fees: Fees,

    /// Total of enter_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub enter_fees_total: FeeAmount,

    /// Enter average price excluding the entry_fees_total.
    pub enter_avg_price_gross: f64,

    /// abs(Quantity) * enter_avg_price_gross.
    pub enter_value_gross: f64,

    /// All fees types incurred from exiting a [`Position`], and their associated [`FeeAmount`].
    pub exit_fees: Fees,

    /// Total of exit_fees incurred. Sum of every [`FeeAmount`] in [`Fees`] when entering a [`Position`].
    pub exit_fees_total: FeeAmount,

    /// Exit average price excluding the exit_fees_total.
    pub exit_avg_price_gross: f64,

    /// abs(Quantity) * exit_avg_price_gross.
    pub exit_value_gross: f64,

    /// Symbol current close price.
    pub current_symbol_price: f64,

    /// abs(Quantity) * current_symbol_price.
    pub current_value_gross: f64,

    /// Unrealised P&L whilst the [`Position`] is open.
    pub unrealised_profit_loss: f64,

    /// Realised P&L after the [`Position`] has closed.
    pub realised_profit_loss: f64,
}

pub fn determine_position_id(exgine_id: Uuid, exchange: &Exchange, instrument: &Instrument) -> PositionId {
    format!("{}_{}_{}_position", exgine_id, exchange, instrument)
}

impl Position {
    /// Returns a [`PositionBuilder`] instance.
    pub fn builder() -> PositionBuilder {
        PositionBuilder::new()
    }

    /// Calculates the [`Position::enter_avg_price_gross`] or [`Position::exit_avg_price_gross`] of
    /// a [`FillEvent`].
    pub fn calculate_avg_price_gross(fill: &FillEvent) -> f64 {
        (fill.fill_value_gross / fill.quantity).abs()
    }

    /// Determine the [`Position`] entry [`Side`] by analysing the input [`FillEvent`].
    pub fn parse_entry_side(fill: &FillEvent) -> Result<PositionSide, PortfolioError> {
        match fill.decision {
            Decision::Long if fill.quantity.is_sign_positive() => Ok(PositionSide::Buy),
            Decision::Short if fill.quantity.is_sign_negative() => Ok(PositionSide::Sell),
            Decision::CloseLong | Decision::CloseShort => Err(PortfolioError::CannotEnterPositionWithExitFill),
            _ => Err(PortfolioError::ParseEntrySide),
        }
    }

    /// Determines the [`Decision`] required to exit this [`Side`] (Buy or Sell) [`Position`].
    pub fn determine_exit_decision(&self) -> Decision {
        match self.side {
            PositionSide::Buy => Decision::CloseLong,
            PositionSide::Sell => Decision::CloseShort,
        }
    }

    /// Calculate the approximate [`Position::unrealised_profit_loss`] of a [`Position`].
    pub fn calculate_unrealised_profit_loss(&self) -> f64 {
        let approx_total_fees = self.enter_fees_total * 2.0;

        match self.side {
            PositionSide::Buy => self.current_value_gross - self.enter_value_gross - approx_total_fees,
            PositionSide::Sell => self.enter_value_gross - self.current_value_gross - approx_total_fees,
        }
    }

    /// Calculate the exact [`Position::realised_profit_loss`] of a [`Position`].
    pub fn calculate_realised_profit_loss(&self) -> f64 {
        let total_fees = self.enter_fees_total + self.exit_fees_total;

        match self.side {
            PositionSide::Buy => self.exit_value_gross - self.enter_value_gross - total_fees,
            PositionSide::Sell => self.enter_value_gross - self.exit_value_gross - total_fees,
        }
    }

    /// Calculate the PnL return of a closed [`Position`] - assumed [`Position::realised_profit_loss`] is
    /// appropriately calculated.
    pub fn calculate_profit_loss_return(&self) -> f64 {
        self.realised_profit_loss / self.enter_value_gross
    }
}
