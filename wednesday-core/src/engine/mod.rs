
use std::{collections::HashMap, iter::Product, path::Component, sync::Arc, thread, time::Duration};

use parking_lot::Mutex;
use prettytable::Table;
use serde::Serialize;
use tokio::{runtime::Handle, sync::{mpsc, oneshot}};
use tracing::{error, info, warn};
use uuid::Uuid;
use wednesday_model::{events::{DataKind, MarketEvent}, identifiers::{Market, MarketId}};

use crate::{data::FeedGenerator, execution::ExecutionClient, model::{engine_error::EngineError, event::{Event, EventTx, MessageTransmitter}, position::Position}, portfolio::{generator::OrderGenerator, repository::{PositionHandler, StatisticHandler}, updater::{FillUpdater, MarketUpdater}}, statistic::summary::{self, combine, PositionSummariser, TableBuilder}, strategy::SignalGenerator};

use self::{builder::EngineBuilder, commond::EngineCommand, trader::Trader};

pub mod commond;
pub mod trader;
pub mod trader_builder;
pub mod builder;
pub struct EngineComponents<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Send,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub engine_id: Uuid, 
    pub command_rx: mpsc::Receiver<EngineCommand>,
    pub portfolio: Arc<Mutex<Portfolio>>,
    pub traders: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>,
    pub trader_command_txs: HashMap<Market, mpsc::Sender<EngineCommand>>,
    pub statistics_summary: Statistic
}

#[derive(Debug)]
pub struct TradingEngine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: PositionSummariser + Serialize + Send,
    Portfolio: PositionHandler 
        + StatisticHandler<Statistic>
        + MarketUpdater
        + OrderGenerator
        + FillUpdater
        + Send
        + 'static,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send + 'static,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub(crate) engine_id: Uuid,
    pub(crate) command_rx: mpsc::Receiver<EngineCommand>,
    pub(crate) portfolio: Arc<Mutex<Portfolio>>,
    pub(crate) traders: Vec<Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>>,
    pub(crate) trader_command_txs: HashMap<Market, mpsc::Sender<EngineCommand>>,
    pub(crate) statistics_summary: Statistic
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    TradingEngine<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Send + 'static,
    Statistic: PositionSummariser + TableBuilder + Serialize + Send + 'static,
    Portfolio: PositionHandler 
        + StatisticHandler<Statistic>
        + MarketUpdater
        + OrderGenerator
        + FillUpdater
        + Send
        + 'static,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send + 'static,
    Execution: ExecutionClient + Send + 'static,
{
    pub fn new(
        component: EngineComponents<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    ) -> Self {
        info!(
            engine_id = &*format!("{}", component.engine_id),
            "constructed new engine context"
        );
        Self {
            engine_id: component.engine_id,
            command_rx: component.command_rx,
            portfolio: component.portfolio,
            traders: component.traders,
            trader_command_txs: component.trader_command_txs,
            statistics_summary: component.statistics_summary
        }
    }

    pub fn builder() -> EngineBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    {
        EngineBuilder::new()
    }

    pub async fn run(mut self) {
        let mut notify_trader_stopped = self.run_traders().await;

        loop {
            tokio::select! {
                _ = notify_trader_stopped.recv() => {
                    info!("all traders have stopped, shutting down engine");
                    break;
                },

                command = self.command_rx.recv() => {
                    if let Some(command) = command {
                        match command {
                            EngineCommand::FetchOpenPositions(position_tx) => 
                                { self.fetch_open_positions(position_tx).await;},
                            EngineCommand::Terminate(meesgae) => 
                                { self.terminate_traders(meesgae).await; break; },
                            EngineCommand::ExitAllPositions => 
                                { self.exit_all_positions().await; },
                            EngineCommand::ExitPosition(market) => 
                                { self.exit_position(market).await; }, 
                        }
                    } else {
                        break;
                    }
                } 
            }
        }

        self.generated_session_summary().printstd();
    }

    async fn run_traders(&mut self) -> mpsc::Receiver<bool> {
        let traders = std::mem::take(&mut self.traders);

        let mut thread_handles = Vec::with_capacity(traders.len());
        for trader in traders.into_iter() {
            let handle = thread::spawn(move || trader.run());
            thread_handles.push(handle);
        }
        // Create Channel to notify the Engine when the Traders have stopped organically
        let (notify_tx, notify_rx) = mpsc::channel(1);

        tokio::spawn(async move {
            for handle in thread_handles {
                if let Err(err) = handle.join() {
                    error!(
                        error = &*format!("{:?}", err),
                        "Trader thread has panicked during execution"
                    )
                }
            }
            notify_tx.send(true).await.unwrap();
        });
        notify_rx
    }

    async fn fetch_open_positions(
        &self,
        positions_tx: oneshot::Sender<Result<Vec<Position>, EngineError>>,
    ) {
        let open_positions = self
            .portfolio
            .lock()
            .get_open_positions(self.engine_id, self.trader_command_txs.keys())
            .map_err(EngineError::RepositoryInteractionError);

        if positions_tx.send(open_positions).is_err() {
            warn!(
                why = "oneshot receiver dropped",
                "can not action EngineCommand::FetchOpenPositions"
            );
        }
    }

    async fn terminate_traders(&self, message: String) {
        self.exit_all_positions().await;
        
        tokio::time::sleep(Duration::from_secs(1)).await;
        for (market, command_tx) in self.trader_command_txs.iter() {
            if command_tx
                .send(EngineCommand::Terminate(message.clone()))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market),
                    why = "dropped receiver",
                    "failed to send EngineCommand:::Terminate to Trader command_rx"
                );
            }
        }
    }

    async fn exit_all_positions(&self) {
        for (market, command_tx) in self.trader_command_txs.iter() {
            if command_tx
                .send(EngineCommand::ExitPosition(market.clone()))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market),
                    why = "dropped receiver",
                    "failed to send EngineCommand::ExitAllPositions to Trader command_rx"
                );
            }
        }
    }

    async fn exit_position(&self, market: Market) {
        if let Some((market_ref, command_tx)) = self.trader_command_txs.get_key_value(&market) {
            if command_tx
                .send(EngineCommand::ExitPosition(market))
                .await
                .is_err()
            {
                error!(
                    market = &*format!("{:?}", market_ref),
                    why = "dropped receiver",
                    "failed to send EngineCommand::ExitPosition to Trader command_rx"
                );
            }
        } else {
            warn!(
                market = &*format!("{:?}", market),
                why = "Engine has no trader_command_tx associated with provided Market",
                "failed to exit position"
            );
        }
    }

    fn generated_session_summary(mut self) -> Table {
        let stats_per_market = self.trader_command_txs
            .into_keys()
            .filter_map(|market| {
                let market_id = MarketId::from(&market);

                match self.portfolio
                    .lock()
                    .get_statistics(&market_id)
                {
                    Ok(statistic) => Some((market_id.0, statistic)),
                    Err(err) => {
                        error!(
                            ?err,
                            ?market,
                            "failed to get Market statistics when generating trading session summary"
                        );
                        None
                    }
                }
            }
        );

        self.portfolio
            .lock()
            .get_exited_positions(self.engine_id)
            .map(|exited_positions| {
                self.statistics_summary.generate_summary(&exited_positions);
            })
            .unwrap_or_else(|err| {
                warn!(
                    ?err,
                    why = "failed to get extied Positions from Portfolio's repository",
                    "failed to generate Statistics summary for trading session"
                );
            }
        );
        
        combine(
            stats_per_market.chain([("Total".to_owned(), self.statistics_summary)]),
        )
    }
}