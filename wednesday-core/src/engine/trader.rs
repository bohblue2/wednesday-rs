use std::{collections::VecDeque, marker::PhantomData, sync::Arc};

use parking_lot::Mutex;
use serde::Serialize;
use tokio::{io::Empty, sync::mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;
use wednesday_model::{events::{DataKind, MarketEvent}, identifiers::Market, order};

use crate::{data::FeedGenerator, execution::ExecutionClient, model::{engine_error::EngineError, enums::Feed, event::{Event, MessageTransmitter}, signal::SignalForceExit}, portfolio::{generator::OrderGenerator, updater::{FillUpdater, MarketUpdater}}, strategy::SignalGenerator};

use super::{commond::EngineCommand, trader_builder::TraderBuilder};

pub struct TraderComponents<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: FeedGenerator<MarketEvent<DataKind>>,
    Strategy: SignalGenerator,
    Execution: ExecutionClient,
{
    pub engine_id: Uuid,
    pub market: Market,
    pub command_rx: mpsc::Receiver<EngineCommand>,
    pub event_tx: EventTx,
    pub portfolio: Arc<Mutex<Portfolio>>,
    pub data: Data,
    pub strategy: Strategy,
    pub execution: Execution,
    _statistic_marker: PhantomData<Statistic>
}

#[derive(Debug)]
pub struct Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event>,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub(crate) engine_id: Uuid,
    pub(crate) market: Market,
    pub(crate) command_rx: mpsc::Receiver<EngineCommand>,
    // [`Event`] transmitter for sending every [`Event`] the [`Trader`] encounters to an external 
    // sink.
    pub(crate) event_tx: EventTx,
    // Queue for storing [`Event`]s used by the trading loop in the run() method.
    pub(crate) event_q: VecDeque<Event>,
    pub(crate) portfolio: Arc<Mutex<Portfolio>>,
    pub(crate) data: Data,
    pub(crate) strategy: Strategy,
    pub(crate) execution: Execution,
    pub(crate) _statistic_marker: PhantomData<Statistic>
}

impl<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    Trader<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
where
    EventTx: MessageTransmitter<Event> + Send,
    Statistic: Serialize + Send,
    Portfolio: MarketUpdater + OrderGenerator + FillUpdater + Send,
    Data: FeedGenerator<MarketEvent<DataKind>> + Send,
    Strategy: SignalGenerator + Send,
    Execution: ExecutionClient + Send,
{
    pub fn new(
        components: TraderComponents<EventTx, Statistic, Portfolio, Data, Strategy, Execution>
    ) -> Self {
        info!(
            engine_id = components.engine_id.to_string(),
            market = ?components.market,
        );

        Self {
            engine_id: components.engine_id,
            market: components.market,
            command_rx: components.command_rx,
            event_tx: components.event_tx,
            event_q: VecDeque::with_capacity(4),
            portfolio: components.portfolio,
            data: components.data,
            strategy: components.strategy,
            execution: components.execution,
            _statistic_marker: PhantomData::default(),
        }
    }

    pub fn builder() -> TraderBuilder<EventTx, Statistic, Portfolio, Data, Strategy, Execution> {
        TraderBuilder::new()
    }

    pub fn run(mut self) {
        'trading: loop {
            // NOTE: 여기에 tick interval 이나 Clock 을 설정해두면 될듯.

            // debug!(
            //     engine_id = &*self.engine_id.to_string(),
            //     market = &*format!("{:?}", self.market),
            //     "Trader trading loop started"
            // );

            // Check for new remote commands before continuing to generate another MarketEvent
            while let Some(command) = self.receive_remote_command() {
                match command {
                    EngineCommand::Terminate(reasone) => { break 'trading }
                    EngineCommand::ExitPosition(market) => {
                        self.event_q
                            .push_back(Event::SignalForceExit(SignalForceExit::from(market)))
                    }
                    // otherwise => continue
                    _ => continue
                }
            }

            // if the Feed<MarketEvent> yield, populate event_q with the next MarketEvent
            match self.data.next() {
                Feed::Next(market) => {
                    // NOTE: This is where the MarketEvent is generated, but cloned()
                    // we need to figure out how to avoid this clone
                    self.event_tx.send(Event::Market(market.clone()));
                    self.event_q.push_back(Event::Market(market));
                }
                Feed::Unhealthy => {
                    warn!(
                        engine_id = %self.engine_id,
                        market = ?self.market,
                        action = "continuing while waiting for healthy Feed",
                        "MarketFeed unhealthy"
                    );
                    continue 'trading;
                },
                Feed::Finished => { break 'trading }, 
            }
            
            // Handle Events in the event_q
            // '--> While loop will break when event_q is empty and requires another MarketEvent
            // NOTE: Maybe we need implement state transition machine to handle the events. 
            // because the current implementation is not clear. we do not know the order of the
            // events and how they are handled.
            while let Some(event) = self.event_q.pop_front() {
                match event {
                    Event::Market(market) => {
                        if let Some(signal) = self.strategy.generate_signal(&market) {
                            self.event_tx.send(Event::Signal(signal.clone()));
                            self.event_q.push_back(Event::Signal(signal));
                        }

                        if let Some(position_update) = self
                            .portfolio
                            .lock()
                            .update_from_market(&market)
                            .expect("failed to update portfolio from market")
                        {
                            self.event_tx.send(Event::PositionUpdate(position_update));
                        }
                    }
                    Event::Signal(signal) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .generate_order(&signal)
                            .expect("failed to generate order")
                        {
                            // NOTE: Clone() occurs here, we need to figure out how to avoid this
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            // self.execution.send_order(order);
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }
                    Event::SignalForceExit(signal_force_exit) => {
                        if let Some(order) = self
                            .portfolio
                            .lock()
                            .generate_exit_order(&signal_force_exit)
                            .expect("failed to generate forced exit order")
                        {
                            // NOTE: Clone() occurs here, we need to figure out how to avoid this
                            self.event_tx.send(Event::OrderNew(order.clone()));
                            // self.execution.send_order(order);
                            self.event_q.push_back(Event::OrderNew(order));
                        }
                    }
                    Event::OrderNew(order) => {
                        let fill = self
                            .execution
                            .generate_fill(&order)
                            .expect("failed to generate fill");
                        
                        self.event_tx.send(Event::Fill(fill.clone()));
                        self.event_q.push_back(Event::Fill(fill));
                    }
                    Event::Fill(fill) => {
                        let fill_side_effect_events = self
                            .portfolio
                            .lock()
                            .update_from_fill(&fill)
                            .expect("failed to update Portfolio from fill");
                        
                        self.event_tx.send_many(fill_side_effect_events);
                    }
                    _ => { debug!(
                        log_meesage = "unhandled event", 
                        _event = &*format!("{:?}", event))
                    }                    
                }
            }

            // debug!(
            //     engine_id = &*self.engine_id.to_string(),
            //     market = &*format!("{:?}", self.market),
            //     "Trader trading loop stopped"
            // );
        }
    }

    fn receive_remote_command(&mut self) -> Option<EngineCommand> {
        match self.command_rx.try_recv() {
            Ok(command) => {
                debug!(
                    engine_id = &*self.engine_id.to_string(),
                    market = &*format!("{:?}", self.market),
                    command = &*format!("{:?}", command),
                );
                Some(command)
            },
            Err(err) => match err {
                mpsc::error::TryRecvError::Empty => None, 
                mpsc::error::TryRecvError::Disconnected => {
                    warn!(
                        action = "synthesising a Command::Terminate",
                        "remote Command transmitter has been dropped"
                    );
                    Some(EngineCommand::Terminate(
                        "remote command transmitter dropped".to_owned(),
                    ))
                }
            }
        }
    }
}

