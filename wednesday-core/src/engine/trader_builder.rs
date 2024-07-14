use std::{collections::VecDeque, marker::PhantomData, sync::Arc};

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;
use wednesday_model::{events::{DataKind, MarketEvent}, identifiers::Market};

use crate::{data::FeedGenerator, execution::ExecutionClient, model::{engine_error::EngineError, event::{Event, MessageTransmitter}}, portfolio::{generator::OrderGenerator, updater::{FillUpdater, MarketUpdater}}, strategy::SignalGenerator};

use super::{commond::EngineCommand, trader::Trader};

#[derive(Debug)]
pub struct TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: FeedGenerator<MarketEvent<DataKind>>,
    Strategy: SignalGenerator,
    Execution: ExecutionClient,
{
    engine_id: Option<Uuid>,
    market: Option<Market>,
    command_rx: Option<mpsc::Receiver<EngineCommand>>,
    event_tx: Option<EventTx>,
    portfolio: Option<Arc<Mutex<Portfolio>>>,
    data: Option<Data>,
    strategy: Option<Strategy>,
    execution: Option<Execution>,
    _statistic_marker: Option<PhantomData<Statistic>>
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub fn new() -> Self {
        Self {
            engine_id: None,
            market: None,
            command_rx: None,
            event_tx: None,
            portfolio: None,
            data: None,
            strategy: None,
            execution: None,
            _statistic_marker: None,
        }
    }

    pub fn engine_id(self, value: Uuid) -> Self {
        Self {
            engine_id: Some(value),
            ..self
        }
    }

    pub fn market(self, value: Market) -> Self {
        Self {
            market: Some(value),
            ..self
        }
    }

    pub fn command_rx(self, value: mpsc::Receiver<EngineCommand>) -> Self {
        Self {
            command_rx: Some(value),
            ..self
        }
    }

    pub fn event_tx(self, value: EventTx) -> Self {
        Self {
            event_tx: Some(value),
            ..self
        }
    }

    pub fn portfolio(self, value: Arc<Mutex<Portfolio>>) -> Self {
        Self {
            portfolio: Some(value),
            ..self
        }
    }

    pub fn data(self, value: Data) -> Self {
        Self {
            data: Some(value),
            ..self
        }
    }

    pub fn strategy(self, value: Strategy) -> Self {
        Self {
            strategy: Some(value),
            ..self
        }
    }

    pub fn execution(self, value: Execution) -> Self {
        Self {
            execution: Some(value),
            ..self
        }
    }

    pub fn build(
        self,
    ) -> Result<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>, EngineError> {
        Ok(Trader {
            engine_id: self
                .engine_id
                .ok_or(EngineError::BuilderIncomplete("engine_id"))?,
            market: self
                .market
                .ok_or(EngineError::BuilderIncomplete("market"))?,
            command_rx: self
                .command_rx
                .ok_or(EngineError::BuilderIncomplete("command_rx"))?,
            event_tx: self
                .event_tx
                .ok_or(EngineError::BuilderIncomplete("event_tx"))?,
            event_q: VecDeque::with_capacity(2),
            portfolio: self
                .portfolio
                .ok_or(EngineError::BuilderIncomplete("portfolio"))?,
            data: self.data.ok_or(EngineError::BuilderIncomplete("data"))?,
            strategy: self
                .strategy
                .ok_or(EngineError::BuilderIncomplete("strategy"))?,
            execution: self
                .execution
                .ok_or(EngineError::BuilderIncomplete("execution"))?,
            _statistic_marker: PhantomData::default(),
        })
    }
}
