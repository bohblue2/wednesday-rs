use wednesday_model::{identifiers::Exchange, instruments::Instrument};

use crate::model::{
    fee::{FeeAmount, Fees},
    portfolio_error::PortfolioError,
};

use super::{Position, PositionId, PositionMeta, PositionSide};

/// Builder to construct [`Position`] instances.
#[derive(Debug, Default)]
pub struct PositionBuilder {
    pub position_id: Option<PositionId>,
    pub exchange: Option<Exchange>,
    pub instrument: Option<Instrument>,
    pub meta: Option<PositionMeta>,
    pub side: Option<PositionSide>,
    pub quantity: Option<f64>,
    pub enter_fees: Option<Fees>,
    pub enter_fees_total: Option<FeeAmount>,
    pub enter_avg_price_gross: Option<f64>,
    pub enter_value_gross: Option<f64>,
    pub exit_fees: Option<Fees>,
    pub exit_fees_total: Option<FeeAmount>,
    pub exit_avg_price_gross: Option<f64>,
    pub exit_value_gross: Option<f64>,
    pub current_symbol_price: Option<f64>,
    pub current_value_gross: Option<f64>,
    pub unrealised_profit_loss: Option<f64>,
    pub realised_profit_loss: Option<f64>,
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn position_id(self, value: PositionId) -> Self {
        Self {
            position_id: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: Exchange) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn instrument(self, value: Instrument) -> Self {
        Self {
            instrument: Some(value),
            ..self
        }
    }

    pub fn meta(self, value: PositionMeta) -> Self {
        Self { meta: Some(value), ..self }
    }

    pub fn side(self, value: PositionSide) -> Self {
        Self { side: Some(value), ..self }
    }

    pub fn quantity(self, value: f64) -> Self {
        Self {
            quantity: Some(value),
            ..self
        }
    }

    pub fn enter_fees(self, value: Fees) -> Self {
        Self {
            enter_fees: Some(value),
            ..self
        }
    }

    pub fn enter_fees_total(self, value: FeeAmount) -> Self {
        Self {
            enter_fees_total: Some(value),
            ..self
        }
    }

    pub fn enter_avg_price_gross(self, value: f64) -> Self {
        Self {
            enter_avg_price_gross: Some(value),
            ..self
        }
    }

    pub fn enter_value_gross(self, value: f64) -> Self {
        Self {
            enter_value_gross: Some(value),
            ..self
        }
    }

    pub fn exit_fees(self, value: Fees) -> Self {
        Self {
            exit_fees: Some(value),
            ..self
        }
    }

    pub fn exit_fees_total(self, value: FeeAmount) -> Self {
        Self {
            exit_fees_total: Some(value),
            ..self
        }
    }

    pub fn exit_avg_price_gross(self, value: f64) -> Self {
        Self {
            exit_avg_price_gross: Some(value),
            ..self
        }
    }

    pub fn exit_value_gross(self, value: f64) -> Self {
        Self {
            exit_value_gross: Some(value),
            ..self
        }
    }

    pub fn current_symbol_price(self, value: f64) -> Self {
        Self {
            current_symbol_price: Some(value),
            ..self
        }
    }

    pub fn current_value_gross(self, value: f64) -> Self {
        Self {
            current_value_gross: Some(value),
            ..self
        }
    }

    pub fn unrealised_profit_loss(self, value: f64) -> Self {
        Self {
            unrealised_profit_loss: Some(value),
            ..self
        }
    }

    pub fn realised_profit_loss(self, value: f64) -> Self {
        Self {
            realised_profit_loss: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<Position, PortfolioError> {
        Ok(Position {
            position_id: self.position_id.ok_or(PortfolioError::BuilderIncomplete("position_id"))?,
            exchange: self.exchange.ok_or(PortfolioError::BuilderIncomplete("exchange"))?,
            instrument: self.instrument.ok_or(PortfolioError::BuilderIncomplete("instrument"))?,
            meta: self.meta.ok_or(PortfolioError::BuilderIncomplete("meta"))?,
            side: self.side.ok_or(PortfolioError::BuilderIncomplete("side"))?,
            quantity: self.quantity.ok_or(PortfolioError::BuilderIncomplete("quantity"))?,
            enter_fees: self.enter_fees.ok_or(PortfolioError::BuilderIncomplete("enter_fees"))?,
            enter_fees_total: self.enter_fees_total.ok_or(PortfolioError::BuilderIncomplete("enter_fees_total"))?,
            enter_avg_price_gross: self
                .enter_avg_price_gross
                .ok_or(PortfolioError::BuilderIncomplete("enter_avg_price_gross"))?,
            enter_value_gross: self
                .enter_value_gross
                .ok_or(PortfolioError::BuilderIncomplete("enter_value_gross"))?,
            exit_fees: self.exit_fees.ok_or(PortfolioError::BuilderIncomplete("exit_fees"))?,
            exit_fees_total: self.exit_fees_total.ok_or(PortfolioError::BuilderIncomplete("exit_fees_total"))?,
            exit_avg_price_gross: self
                .exit_avg_price_gross
                .ok_or(PortfolioError::BuilderIncomplete("exit_avg_price_gross"))?,
            exit_value_gross: self.exit_value_gross.ok_or(PortfolioError::BuilderIncomplete("exit_value_gross"))?,
            current_symbol_price: self
                .current_symbol_price
                .ok_or(PortfolioError::BuilderIncomplete("current_symbol_price"))?,
            current_value_gross: self
                .current_value_gross
                .ok_or(PortfolioError::BuilderIncomplete("current_value_gross"))?,
            unrealised_profit_loss: self
                .unrealised_profit_loss
                .ok_or(PortfolioError::BuilderIncomplete("unrealised_profit_loss"))?,
            realised_profit_loss: self
                .realised_profit_loss
                .ok_or(PortfolioError::BuilderIncomplete("realised_profit_loss"))?,
        })
    }
}
