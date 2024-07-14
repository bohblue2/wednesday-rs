use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;
use wednesday_model::{
    events::{DataKind, MarketEvent},
    identifiers::Market,
};

use crate::{
    data::FeedGenerator,
    execution::ExecutionClient,
    model::{
        engine_error::EngineError,
        event::{Event, MessageTransmitter},
    },
    portfolio::{
        generator::OrderGenerator,
        repository::{PositionHandler, StatisticHandler},
        updater::{FillUpdater, MarketUpdater},
    },
    statistic::summary::PositionSummariser,
    strategy::SignalGenerator,
};

use super::{commond::EngineCommand, trader::Trader, TradingEngine};

/// Builder to construct [`Engine`] instances.
#[derive(Debug, Default)]
pub struct EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    engine_id: Option<Uuid>,
    command_rx: Option<mpsc::Receiver<EngineCommand>>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    traders: Option<Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>>,
    trader_command_txs: Option<HashMap<Market, mpsc::Sender<EngineCommand>>>,
    statistics_summary: Option<Statistic>,
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution> EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: PositionSummariser + Serialize + Send,
    Portfolio: PositionHandler + StatisticHandler<Statistic> + MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub fn new() -> Self {
        Self {
            engine_id: None,
            command_rx: None,
            portfolio: None,
            traders: None,
            trader_command_txs: None,
            statistics_summary: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn command_rx(self, value: mpsc::Receiver<EngineCommand>) -> Self {
        Self {
            command_rx: Some(value),
            ..self
        }
    }

    pub fn portfolio(self, value: Arc<Mutex<Portfolio>>) -> Self {
        Self {
            portfolio: Some(value),
            ..self
        }
    }

    pub fn traders(self, value: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>) -> Self {
        Self {
            traders: Some(value),
            ..self
        }
    }

    pub fn trader_command_txs(self, value: HashMap<Market, mpsc::Sender<EngineCommand>>) -> Self {
        Self {
            trader_command_txs: Some(value),
            ..self
        }
    }

    pub fn statistics_summary(self, value: Statistic) -> Self {
        Self {
            statistics_summary: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<TradingEngine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
        Ok(TradingEngine {
            engine_id: self.engine_id.ok_or(EngineError::BuilderIncomplete("engine_id"))?,
            command_rx: self.command_rx.ok_or(EngineError::BuilderIncomplete("command_rx"))?,
            portfolio: self.portfolio.ok_or(EngineError::BuilderIncomplete("portfolio"))?,
            traders: self.traders.ok_or(EngineError::BuilderIncomplete("traders"))?,
            trader_command_txs: self
                .trader_command_txs
                .ok_or(EngineError::BuilderIncomplete("trader_command_txs"))?,
            statistics_summary: self
                .statistics_summary
                .ok_or(EngineError::BuilderIncomplete("statistics_summary"))?,
        })
    }
}
