use wednesday_model::events::{DataKind, MarketEvent};

use crate::model::{event::Event, fill_event::FillEvent, portfolio_error::PortfolioError, position::updater::PositionUpdate};

pub trait MarketUpdater {
    fn update_from_market(&mut self, market_meta: &MarketEvent<DataKind>) -> Result<Option<PositionUpdate>, PortfolioError>;
}

pub trait FillUpdater {
    fn update_from_fill(&mut self, fill_event: &FillEvent) -> Result<Vec<Event>, PortfolioError>;
}
