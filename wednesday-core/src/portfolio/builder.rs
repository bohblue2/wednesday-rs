use std::marker::PhantomData;

use uuid::Uuid;
use wednesday_model::identifiers::Market;

use crate::{model::portfolio_error::PortfolioError, oms::{allocator::OrderAllocator, evaluator::OrderEvaluator}, statistic::summary::{Initialiser, PositionSummariser}};

use super::{repository::{BalanceHandler, PositionHandler, StatisticHandler}, MetaPortfolio};


pub struct MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser
{
    engine_id: Option<Uuid>,
    markets: Option<Vec<Market>>,
    starting_cash: Option<f64>,
    repository: Option<Repository>,
    allocation_manager: Option<Allocator>,
    risk_manager: Option<RiskManager>,
    statistic_config: Option<Statistic::Config>,
    _statistic_marker: Option<PhantomData<Statistic>>,
}


impl<Repository, Allocator, RiskManager, Statistic>
    MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    pub fn new() -> Self {
        Self {
            engine_id: None,
            markets: None,
            starting_cash: None,
            repository: None,
            allocation_manager: None,
            risk_manager: None,
            statistic_config: None,
            _statistic_marker: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn markets(self, value: Vec<Market>) -> Self {
        Self {
            markets: Some(value),
            ..self
        }
    }

    pub fn starting_cash(self, value: f64) -> Self {
        Self {
            starting_cash: Some(value),
            ..self
        }
    }

    pub fn repository(self, value: Repository) -> Self {
        Self {
            repository: Some(value),
            ..self
        }
    }

    pub fn allocation_manager(self, value: Allocator) -> Self {
        Self {
            allocation_manager: Some(value),
            ..self
        }
    }

    pub fn risk_manager(self, value: RiskManager) -> Self {
        Self {
            risk_manager: Some(value),
            ..self
        }
    }

    pub fn statistic_config(self, value: Statistic::Config) -> Self {
        Self {
            statistic_config: Some(value),
            ..self
        }
    }

    pub fn build_and_init(
        self,
    ) -> Result<MetaPortfolio<Repository, Allocator, RiskManager, Statistic>, PortfolioError> {
        // Construct Portfolio
        let mut portfolio = MetaPortfolio {
            engine_id: self
                .engine_id
                .ok_or(PortfolioError::BuilderIncomplete("engine_id"))?,
            repository: self
                .repository
                .ok_or(PortfolioError::BuilderIncomplete("repository"))?,
            allocation_manager: self
                .allocation_manager
                .ok_or(PortfolioError::BuilderIncomplete("allocation_manager"))?,
            risk_manager: self
                .risk_manager
                .ok_or(PortfolioError::BuilderIncomplete("risk_manager"))?,
            _statistic_marker: PhantomData::default(),
        };

        // Persist initial state in the Repository
        portfolio.bootstrap_repository(
            self.starting_cash
                .ok_or(PortfolioError::BuilderIncomplete("starting_cash"))?,
            &self
                .markets
                .ok_or(PortfolioError::BuilderIncomplete("markets"))?,
            self.statistic_config
                .ok_or(PortfolioError::BuilderIncomplete("statistic_config"))?,
        )?;

        Ok(portfolio)
    }
}