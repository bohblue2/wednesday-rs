use std::{collections::HashMap, marker::PhantomData, os::macos::raw::stat};

use chrono::Utc;
use futures::{future::ok, stream::All};
use ta::indicators::Minimum;
use tracing::info;
use uuid::Uuid;
use wednesday_model::{enums::OrderType, events::{DataKind, MarketEvent}, identifiers::{Market, MarketId}};

use crate::{model::{balance::{self, Balance}, decision::Decision, event::Event, fill_event::FillEvent, market_meta::{self, MarketMeta}, order_event::OrderEvent, portfolio_error::PortfolioError, position::{self, determine_position_id, enterer::PositionEnterer, exiter::PositionExiter, updater::{PositionUpdate, PositionUpdater}, Position, PositionSide}, repository_error::RepositoryError, signal::SignalStrength}, oms::{allocator::OrderAllocator, evaluator::OrderEvaluator}, statistic::{self, summary::{Initialiser, PositionSummariser}}};

use self::{builder::MetaPortfolioBuilder, generator::OrderGenerator, repository::{BalanceHandler, PositionHandler, StatisticHandler}, updater::{FillUpdater, MarketUpdater}};

pub mod updater;
pub mod generator;
pub mod repository;
pub mod builder;

pub struct PortfolioComponents<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    // Allocator: PositionAllocator + Send,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    pub engine_id: Uuid,
    pub markets: Vec<Market>, 
    pub repository: Repository,
    pub allocator: Allocator, // Allocation Manager
    pub risk: RiskManager, // Risk Manager
    pub starting_cash: f64,
    /// Configuration used to initialise the Statistics for every Market's performance tracked by a
    /// [`MetaPortfolio`].
    pub statistic_config: Statistic::Config,
    pub _statistic_marker: PhantomData<Statistic>
}

pub struct MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    engine_id: Uuid,
    repository: Repository,
    allocation_manager: Allocator,
    risk_manager: RiskManager,
    _statistic_marker: PhantomData<Statistic>
}

impl<Repository, Allocator, RiskManager, Statistic> MarketUpdater 
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn update_from_market(
        &mut self,
        market_meta: &MarketEvent<DataKind>,
    ) -> Result<Option<PositionUpdate>, PortfolioError> {
        let position_id = determine_position_id(
            self.engine_id,
            &market_meta.exchange, 
            &market_meta.instrument
        );

        if let Some(mut position) = self.repository.get_open_position(&position_id)? {
            if let Some(position_update) = position.update(market_meta) {
                self.repository.set_open_position(position)?;
                return Ok(Some(position_update))
            }
        }
        Ok(None)
    }
}

impl<Repository, Allocator, RiskManager, Statistic> OrderGenerator 
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn generate_order(
        &mut self, 
        signal: &crate::model::signal::Signal
    ) -> Result<Option<crate::model::order_event::OrderEvent>, PortfolioError> {
        let position_id = determine_position_id(
            self.engine_id,
            &signal.exchange,
            &signal.instrument
        );

        let position = self.repository.get_open_position(&position_id)?;

        if position.is_none() && self.no_cash_to_enter_new_position()? {
            return Ok(None)
        }

        let position = position.as_ref();
        let (signal_decision, signal_strength) = 
            match parse_signal_decisions(&position, &signal.signals) {
                None => return Ok(None),
                Some(net_signal) => net_signal
            };
        
        let mut order = OrderEvent {
            timestamp: Utc::now(),
            exchange: signal.exchange.clone(),
            instrument: signal.instrument.clone(),
            market_meta: signal.market_meta,
            decision: *signal_decision,
            quantity: 0.1,
            order_type: OrderType::Limit,
        };

        self.allocation_manager
            .allocate_order(&mut order, position, *signal_strength);
        Ok(self.risk_manager.evaluate_order(order))
    }

    fn generate_exit_order(
        &mut self,
        signal: &crate::model::signal::SignalForceExit
    ) -> Result<Option<crate::model::order_event::OrderEvent>, PortfolioError> {
        let position_id = determine_position_id(
            self.engine_id,
            &signal.exchange,
            &signal.instrument
        );

        let position = match self.repository.get_open_position(&position_id)? {
            None => {
                info!(
                    position_id = &*position_id,
                    outcome = "no forced exit OrderEvent generated",
                    "cannot generate forced exit OrderEvent for a Position that isn't open"
                );
                return Ok(None);
            }
            Some(position) => position,
        };

        Ok(Some(OrderEvent {
            timestamp: Utc::now(),
            exchange: signal.exchange.clone(),
            instrument: signal.instrument.clone(),
            market_meta: MarketMeta {
                close: position.current_symbol_price,
                timestamp: position.meta.update_timestamp
            },
            decision: position.determine_exit_decision(),
            quantity: 0.0 - position.quantity,
            order_type: OrderType::Market,
        }))
    }
}

impl<Repository, Allocator, RiskManager, Statistic> FillUpdater
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn update_from_fill(
        &mut self,
        fill: &FillEvent
    ) -> Result<Vec<Event>, PortfolioError> {
        let mut generated_events: Vec<Event> = Vec::with_capacity(2);

        let mut balance = self.repository.get_balance(self.engine_id)?;
        balance.timestamp = fill.timestamp;

        let position_id = determine_position_id(
            self.engine_id,
            &fill.exchange,
            &fill.instrument
        );

        // NOTE: 수량 체크하고 remove_position() 해야하지 않나 ?
        // NOTE: Position 수량 변경하고 balance 업데이트 치는 이부분 뭔가 다시 제대로 계산해야할 것 같은데.
        match self.repository.remove_position(&position_id)? {
            // EXIT SCENARIO - FillEvent for Symbol-Exchange combination with open Position
            Some(mut position) => {
                // Exit Position (in place mutation), & add the PositionExit event to Vec<Event>
                let position_exit = position.exit(balance, fill)?;
                generated_events.push(Event::PositionExit(position_exit));

                // Update Portfolio balance on Position Exit
                balance.available += position.enter_value_gross 
                    + position.realised_profit_loss 
                    + position.enter_fees_total;

                let market_id = MarketId::new(&fill.exchange, &fill.instrument);
                let mut stats = self.repository.get_statistics(&market_id)?;
                stats.update(&position);

                // Persist exited Position & Update Market statistics in Repository
                self.repository.set_statistics(market_id, stats);
                self.repository.set_exited_position(self.engine_id, position)?;
            },
            // ENTRY SCENARIO - FillEvent for Symbol-Exchange with no position
            None => {
                let position = Position::enter(self.engine_id, fill)?;
                generated_events.push(Event::PositionNew(position.clone()));

                balance.available += -position.enter_value_gross - position.enter_fees_total;
                
                self.repository.set_open_position(position)?; 
            }   
        }
        generated_events.push(Event::Balance(balance));

        self.repository.set_balance(self.engine_id, balance)?;

        Ok(generated_events)
    }
}


impl<Repository, Allocator, RiskManager, Statistic> PositionHandler
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn set_open_position(
        &mut self,
        position: Position
    ) -> Result<(), RepositoryError> {
        self.repository.set_open_position(position)
    }

    fn get_open_position(
        &mut self,
        position_id: &position::PositionId
    ) -> Result<Option<Position>, RepositoryError> {
        self.repository.get_open_position(position_id)
    }

    fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
        &mut self,
        // NOTE: 나중에 engine_id 는 리팩토링 할꺼임.
        _: Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError> {
        self.repository.get_open_positions(self.engine_id, markets)
    }

    fn remove_position(
        &mut self,
        position_id: &position::PositionId
    ) -> Result<Option<Position>, RepositoryError> {
        self.repository.remove_position(position_id)
    }

    fn set_exited_position(
        &mut self,
        _: Uuid,
        position: Position
    ) -> Result<(), RepositoryError> {
        self.repository.set_exited_position(self.engine_id, position)
    }

    fn get_exited_positions(
        &mut self,
        engine_id: Uuid
    ) -> Result<Vec<Position>, RepositoryError> {
        self.repository.get_exited_positions(self.engine_id)
    }
}

impl<Repository, Allocator, RiskManager, Statistic> StatisticHandler<Statistic>
    for MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    fn set_statistics(
        &mut self, 
        market_id: MarketId,
        statistic: Statistic
    ) -> Result<(), RepositoryError> {
        self.repository.set_statistics(market_id, statistic)
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        self.repository.get_statistics(market_id)
    }

}

impl<Repository, Allocator, RiskManager, Statistic>
    MetaPortfolio<Repository, Allocator, RiskManager, Statistic>
where
    Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
    Allocator: OrderAllocator,
    RiskManager: OrderEvaluator,
    Statistic: Initialiser + PositionSummariser,
{
    pub fn init(
        components: PortfolioComponents<Repository, Allocator, RiskManager, Statistic>
    ) -> Result<Self, PortfolioError> {
        let mut portfolio = Self {
            engine_id: components.engine_id,
            repository: components.repository,
            allocation_manager: components.allocator,
            risk_manager: components.risk,
            _statistic_marker: PhantomData::default(),
        };
        
        portfolio.bootstrap_repository(
            components.starting_cash,
            &components.markets,
            components.statistic_config
        )?;
        Ok(portfolio)
    }

    pub fn bootstrap_repository<Markets, Id>(
        &mut self,
        starting_cash: f64,
        markets: Markets,
        statistic_config: Statistic::Config
    ) -> Result<(), PortfolioError>
    where
        Markets: IntoIterator<Item = Id>,
        Id: Into<MarketId>
    {
        self.repository.set_balance(
            self.engine_id,
            Balance {
                timestamp: Utc::now(),
                total: starting_cash,
                available: starting_cash,
            }
        )?;

        markets.into_iter().try_for_each(|market| {
            self.repository
                .set_statistics(market.into(), Statistic::init(statistic_config))
                .map_err(PortfolioError::RepositoryInteraction)
        })
    }

    pub fn builder() -> MetaPortfolioBuilder<Repository, Allocator, RiskManager, Statistic> {
        MetaPortfolioBuilder::new()
    }

    fn no_cash_to_enter_new_position(&mut self) -> Result<bool, PortfolioError> {
        let minimum_threshold = 1.0;
        self.repository
            .get_balance(self.engine_id)
            .map(|balance| balance.available < minimum_threshold)
            .map_err(PortfolioError::RepositoryInteraction)
    }

}

pub fn parse_signal_decisions<'a>(
    position: &'a Option<&Position>,
    signals: &'a HashMap<Decision, SignalStrength>,
) -> Option<(&'a Decision, &'a SignalStrength)> {
    // Determine the presence of signals in the provided signals HashMap
    let signal_close_long = signals.get_key_value(&Decision::CloseLong);
    let signal_long = signals.get_key_value(&Decision::Long);
    let signal_close_short = signals.get_key_value(&Decision::CloseShort);
    let signal_short = signals.get_key_value(&Decision::Short);

    // If an existing Position exists, check for net close signals
    if let Some(position) = position {
        return match position.side {
            PositionSide::Buy if signal_close_long.is_some() => signal_close_long,
            PositionSide::Sell if signal_close_short.is_some() => signal_close_short,
            _ => None,
        };
    }

    // Else check for net open signals
    match (signal_long, signal_short) {
        (Some(signal_long), None) => Some(signal_long),
        (None, Some(signal_short)) => Some(signal_short),
        _ => None,
    }
}


// #[cfg(test)]
// pub mod tests {
//     use super::*;

//     use crate::{
//         execution::Fees,
//         portfolio::{
//             allocator::DefaultAllocator, position::PositionBuilder,
//             repository::error::RepositoryError, risk::DefaultRisk,
//         },
//         statistic::summary::pnl::PnLReturnSummary,
//         strategy::SignalForceExit,
//         test_util::{fill_event, market_event_trade, position, signal},
//     };
//     use barter_integration::model::{
//         instrument::{kind::InstrumentKind, Instrument},
//         Exchange, Side,
//     };

//     #[derive(Default)]
//     struct MockRepository<Statistic> {
//         set_open_position: Option<fn(position: Position) -> Result<(), RepositoryError>>,
//         get_open_position:
//             Option<fn(position_id: &String) -> Result<Option<Position>, RepositoryError>>,
//         get_open_positions: Option<
//             fn(engine_id: Uuid, markets: Vec<&Market>) -> Result<Vec<Position>, RepositoryError>,
//         >,
//         remove_position:
//             Option<fn(engine_id: &String) -> Result<Option<Position>, RepositoryError>>,
//         set_exited_position:
//             Option<fn(engine_id: Uuid, position: Position) -> Result<(), RepositoryError>>,
//         get_exited_positions: Option<fn(engine_id: Uuid) -> Result<Vec<Position>, RepositoryError>>,
//         set_balance: Option<fn(engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError>>,
//         get_balance: Option<fn(engine_id: Uuid) -> Result<Balance, RepositoryError>>,
//         set_statistics:
//             Option<fn(market_id: MarketId, statistic: Statistic) -> Result<(), RepositoryError>>,
//         get_statistics: Option<fn(market_id: &MarketId) -> Result<Statistic, RepositoryError>>,
//         position: Option<PositionBuilder>,
//         balance: Option<Balance>,
//     }

//     impl<Statistic> PositionHandler for MockRepository<Statistic> {
//         fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
//             self.position = Some(
//                 Position::builder()
//                     .side(position.side.clone())
//                     .current_symbol_price(position.current_symbol_price)
//                     .current_value_gross(position.current_value_gross)
//                     .enter_fees_total(position.enter_fees_total)
//                     .enter_value_gross(position.enter_value_gross)
//                     .enter_avg_price_gross(position.enter_avg_price_gross)
//                     .exit_fees_total(position.exit_fees_total)
//                     .exit_value_gross(position.exit_value_gross)
//                     .exit_avg_price_gross(position.exit_avg_price_gross)
//                     .unrealised_profit_loss(position.unrealised_profit_loss)
//                     .realised_profit_loss(position.realised_profit_loss),
//             );
//             self.set_open_position.unwrap()(position)
//         }

//         fn get_open_position(
//             &mut self,
//             position_id: &String,
//         ) -> Result<Option<Position>, RepositoryError> {
//             self.get_open_position.unwrap()(position_id)
//         }

//         fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
//             &mut self,
//             engine_id: Uuid,
//             markets: Markets,
//         ) -> Result<Vec<Position>, RepositoryError> {
//             self.get_open_positions.unwrap()(engine_id, markets.into_iter().collect())
//         }

//         fn remove_position(
//             &mut self,
//             position_id: &String,
//         ) -> Result<Option<Position>, RepositoryError> {
//             self.remove_position.unwrap()(position_id)
//         }

//         fn set_exited_position(
//             &mut self,
//             portfolio_id: Uuid,
//             position: Position,
//         ) -> Result<(), RepositoryError> {
//             self.set_exited_position.unwrap()(portfolio_id, position)
//         }

//         fn get_exited_positions(
//             &mut self,
//             portfolio_id: Uuid,
//         ) -> Result<Vec<Position>, RepositoryError> {
//             self.get_exited_positions.unwrap()(portfolio_id)
//         }
//     }

//     impl<Statistic> BalanceHandler for MockRepository<Statistic> {
//         fn set_balance(
//             &mut self,
//             engine_id: Uuid,
//             balance: Balance,
//         ) -> Result<(), RepositoryError> {
//             self.balance = Some(balance);
//             self.set_balance.unwrap()(engine_id, balance)
//         }

//         fn get_balance(&mut self, engine_id: Uuid) -> Result<Balance, RepositoryError> {
//             self.get_balance.unwrap()(engine_id)
//         }
//     }

//     impl<Statistic> StatisticHandler<Statistic> for MockRepository<Statistic> {
//         fn set_statistics(
//             &mut self,
//             market_id: MarketId,
//             statistic: Statistic,
//         ) -> Result<(), RepositoryError> {
//             self.set_statistics.unwrap()(market_id, statistic)
//         }

//         fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
//             self.get_statistics.unwrap()(market_id)
//         }
//     }

//     fn new_mocked_portfolio<Repository, Statistic>(
//         mock_repository: Repository,
//     ) -> Result<MetaPortfolio<Repository, DefaultAllocator, DefaultRisk, Statistic>, PortfolioError>
//     where
//         Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
//         Statistic: PositionSummariser + Initialiser,
//     {
//         let builder = MetaPortfolio::builder()
//             .engine_id(Uuid::new_v4())
//             .starting_cash(1000.0)
//             .repository(mock_repository)
//             .allocation_manager(DefaultAllocator {
//                 default_order_value: 100.0,
//             })
//             .risk_manager(DefaultRisk {});

//         build_uninitialised_portfolio(builder)
//     }

//     fn build_uninitialised_portfolio<Repository, Statistic>(
//         builder: MetaPortfolioBuilder<Repository, DefaultAllocator, DefaultRisk, Statistic>,
//     ) -> Result<MetaPortfolio<Repository, DefaultAllocator, DefaultRisk, Statistic>, PortfolioError>
//     where
//         Repository: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
//         Statistic: PositionSummariser + Initialiser,
//     {
//         Ok(MetaPortfolio {
//             engine_id: builder
//                 .engine_id
//                 .ok_or(PortfolioError::BuilderIncomplete("engine_id"))?,
//             repository: builder
//                 .repository
//                 .ok_or(PortfolioError::BuilderIncomplete("repository"))?,
//             allocation_manager: builder
//                 .allocation_manager
//                 .ok_or(PortfolioError::BuilderIncomplete("allocation_manager"))?,
//             risk_manager: builder
//                 .risk_manager
//                 .ok_or(PortfolioError::BuilderIncomplete("risk_manager"))?,
//             _statistic_marker: Default::default(),
//         })
//     }

//     fn new_signal_force_exit() -> SignalForceExit {
//         SignalForceExit {
//             time: Utc::now(),
//             exchange: Exchange::from("binance"),
//             instrument: Instrument::from(("eth", "usdt", InstrumentKind::Spot)),
//         }
//     }

//     #[test]
//     fn update_from_market_with_long_position_increasing_in_value() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut input_position = position();
//                 input_position.side = Side::Buy;
//                 input_position.quantity = 1.0;
//                 input_position.enter_fees_total = 3.0;
//                 input_position.current_symbol_price = 100.0;
//                 input_position.current_value_gross = 100.0;
//                 input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
//                 input_position
//             }))
//         });
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input MarketEvent
//         let mut input_market = market_event_trade(Side::Buy);

//         match input_market.kind {
//             // candle.close +100.0 on input_position.current_symbol_price
//             DataKind::Candle(ref mut candle) => candle.close = 200.0,
//             DataKind::Trade(ref mut trade) => trade.price = 200.0,
//             _ => todo!(),
//         };

//         let result_pos_update = portfolio
//             .update_from_market(&input_market)
//             .unwrap()
//             .unwrap();
//         let updated_position = portfolio.repository.position.unwrap();

//         assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
//         assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);

//         // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
//         assert_eq!(
//             updated_position.unrealised_profit_loss.unwrap(),
//             200.0 - 100.0 - 6.0
//         );
//         assert_eq!(
//             result_pos_update.unrealised_profit_loss,
//             200.0 - 100.0 - 6.0
//         );
//     }

//     #[test]
//     fn update_from_market_with_long_position_decreasing_in_value() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut input_position = position();
//                 input_position.side = Side::Buy;
//                 input_position.quantity = 1.0;
//                 input_position.enter_fees_total = 3.0;
//                 input_position.current_symbol_price = 100.0;
//                 input_position.current_value_gross = 100.0;
//                 input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
//                 input_position
//             }))
//         });
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input MarketEvent
//         let mut input_market = market_event_trade(Side::Buy);
//         match input_market.kind {
//             // -50.0 on input_position.current_symbol_price
//             DataKind::Candle(ref mut candle) => candle.close = 50.0,
//             DataKind::Trade(ref mut trade) => trade.price = 50.0,
//             _ => todo!(),
//         };

//         let result_pos_update = portfolio
//             .update_from_market(&input_market)
//             .unwrap()
//             .unwrap();
//         let updated_position = portfolio.repository.position.unwrap();

//         assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
//         assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
//         // Unreal PnL Long = current_value_gross - enter_value_gross - enter_fees_total*2
//         assert_eq!(
//             updated_position.unrealised_profit_loss.unwrap(),
//             50.0 - 100.0 - 6.0
//         );
//         assert_eq!(result_pos_update.unrealised_profit_loss, 50.0 - 100.0 - 6.0);
//     }

//     #[test]
//     fn update_from_market_with_short_position_increasing_in_value() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut input_position = position();
//                 input_position.side = Side::Sell;
//                 input_position.quantity = -1.0;
//                 input_position.enter_fees_total = 3.0;
//                 input_position.current_symbol_price = 100.0;
//                 input_position.current_value_gross = 100.0;
//                 input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
//                 input_position
//             }))
//         });
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input MarketEvent
//         let mut input_market = market_event_trade(Side::Buy);

//         match input_market.kind {
//             // -50.0 on input_position.current_symbol_price
//             DataKind::Candle(ref mut candle) => candle.close = 50.0,
//             DataKind::Trade(ref mut trade) => trade.price = 50.0,
//             _ => todo!(),
//         };

//         let result_pos_update = portfolio
//             .update_from_market(&input_market)
//             .unwrap()
//             .unwrap();
//         let updated_position = portfolio.repository.position.unwrap();

//         assert_eq!(updated_position.current_symbol_price.unwrap(), 50.0);
//         assert_eq!(updated_position.current_value_gross.unwrap(), 50.0);
//         // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
//         assert_eq!(
//             updated_position.unrealised_profit_loss.unwrap(),
//             100.0 - 50.0 - 6.0
//         );
//         assert_eq!(result_pos_update.unrealised_profit_loss, 100.0 - 50.0 - 6.0);
//     }

//     #[test]
//     fn update_from_market_with_short_position_decreasing_in_value() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut input_position = position();
//                 input_position.side = Side::Sell;
//                 input_position.quantity = -1.0;
//                 input_position.enter_fees_total = 3.0;
//                 input_position.current_symbol_price = 100.0;
//                 input_position.current_value_gross = 100.0;
//                 input_position.unrealised_profit_loss = -3.0; // -3.0 from entry fees
//                 input_position
//             }))
//         });
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input MarketEvent
//         let mut input_market = market_event_trade(Side::Buy);

//         match input_market.kind {
//             // +100.0 on input_position.current_symbol_price
//             DataKind::Candle(ref mut candle) => candle.close = 200.0,
//             DataKind::Trade(ref mut trade) => trade.price = 200.0,
//             _ => todo!(),
//         };

//         let result_pos_update = portfolio
//             .update_from_market(&input_market)
//             .unwrap()
//             .unwrap();
//         let updated_position = portfolio.repository.position.unwrap();

//         assert_eq!(updated_position.current_symbol_price.unwrap(), 200.0);
//         assert_eq!(updated_position.current_value_gross.unwrap(), 200.0);
//         // Unreal PnL Short = enter_value_gross - current_value_gross - enter_fees_total*2
//         assert_eq!(
//             updated_position.unrealised_profit_loss.unwrap(),
//             100.0 - 200.0 - 6.0
//         );
//         assert_eq!(
//             result_pos_update.unrealised_profit_loss,
//             100.0 - 200.0 - 6.0
//         );
//     }

//     #[test]
//     fn generate_no_order_with_no_position_and_no_cash() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| Ok(None));
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 0.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let input_signal = signal();

//         let actual = portfolio.generate_order(&input_signal).unwrap();

//         assert!(actual.is_none())
//     }

//     #[test]
//     fn generate_no_order_with_position_and_no_cash() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| Ok(Some(position())));
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 0.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let input_signal = signal();

//         let actual = portfolio.generate_order(&input_signal).unwrap();

//         assert!(actual.is_none())
//     }

//     #[test]
//     fn generate_order_long_with_no_position_and_input_net_long_signal() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| Ok(None));
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 100.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let mut input_signal = signal();
//         input_signal
//             .signals
//             .insert(Decision::Long, SignalStrength(1.0));

//         let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

//         assert_eq!(actual.decision, Decision::Long)
//     }

//     #[test]
//     fn generate_order_short_with_no_position_and_input_net_short_signal() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| Ok(None));
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 100.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let mut input_signal = signal();

//         input_signal
//             .signals
//             .insert(Decision::Short, SignalStrength(1.0));

//         let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

//         assert_eq!(actual.decision, Decision::Short)
//     }

//     #[test]
//     fn generate_order_close_long_with_long_position_and_input_net_close_long_signal() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut position = position();
//                 position.side = Side::Buy;
//                 position
//             }))
//         });
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 100.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let mut input_signal = signal();

//         input_signal
//             .signals
//             .insert(Decision::CloseLong, SignalStrength(1.0));

//         let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

//         assert_eq!(actual.decision, Decision::CloseLong)
//     }

//     #[test]
//     fn generate_order_close_short_with_short_position_and_input_net_close_short_signal() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut position = position();
//                 position.side = Side::Sell;
//                 position
//             }))
//         });
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 100.0,
//                 available: 100.0,
//             })
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let mut input_signal = signal();

//         input_signal
//             .signals
//             .insert(Decision::CloseShort, SignalStrength(1.0));

//         let actual = portfolio.generate_order(&input_signal).unwrap().unwrap();

//         assert_eq!(actual.decision, Decision::CloseShort)
//     }

//     #[test]
//     fn generate_exit_order_with_long_position_open() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut position = position();
//                 position.side = Side::Buy;
//                 position.quantity = 100.0;
//                 position
//             }))
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let input_signal = new_signal_force_exit();

//         // Expect Ok(Some(OrderEvent))
//         let actual = portfolio
//             .generate_exit_order(input_signal)
//             .unwrap()
//             .unwrap();

//         assert_eq!(actual.decision, Decision::CloseLong);
//         assert_eq!(actual.quantity, -100.0);
//         assert_eq!(actual.order_type, OrderType::Market)
//     }

//     #[test]
//     fn generate_exit_order_with_short_position_open() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| {
//             Ok(Some({
//                 let mut position = position();
//                 position.side = Side::Sell;
//                 position.quantity = -100.0;
//                 position
//             }))
//         });
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let input_signal = new_signal_force_exit();

//         // Expect Ok(Some(OrderEvent))
//         let actual = portfolio
//             .generate_exit_order(input_signal)
//             .unwrap()
//             .unwrap();

//         assert_eq!(actual.decision, Decision::CloseShort);
//         assert_eq!(actual.quantity, 100.0);
//         assert_eq!(actual.order_type, OrderType::Market)
//     }

//     #[test]
//     fn generate_no_exit_order_when_no_open_position_to_exit() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_open_position = Some(|_| Ok(None));

//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input SignalEvent
//         let input_signal = new_signal_force_exit();

//         let actual = portfolio.generate_exit_order(input_signal).unwrap();
//         assert!(actual.is_none());
//     }

//     #[test]
//     fn update_from_fill_entering_long_position() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 200.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| Ok(None));
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::Long;
//         input_fill.quantity = 1.0;
//         input_fill.fill_value_gross = 100.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let entered_position = updated_repository.position.unwrap();
//         let updated_cash = updated_repository.balance.unwrap().available;

//         assert!(result.is_ok());
//         assert_eq!(entered_position.side.unwrap(), Side::Buy);
//         assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
//         assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
//         assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
//     }

//     #[test]
//     fn update_from_fill_entering_short_position() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 200.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| Ok(None));
//         mock_repository.set_open_position = Some(|_| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::Short;
//         input_fill.quantity = -1.0;
//         input_fill.fill_value_gross = 100.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let entered_position = updated_repository.position.unwrap();
//         let updated_cash = updated_repository.balance.unwrap().available;

//         assert!(result.is_ok());
//         assert_eq!(entered_position.side.unwrap(), Side::Sell);
//         assert_eq!(entered_position.enter_value_gross.unwrap(), 100.0);
//         assert_eq!(entered_position.enter_fees_total.unwrap(), 3.0);
//         assert_eq!(updated_cash, 200.0 - 100.0 - 3.0); // cash += enter_value_gross - enter_fees
//     }

//     #[test]
//     fn update_from_fill_exiting_long_position_in_profit() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 97.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| {
//             Ok({
//                 Some({
//                     let mut input_position = position();
//                     input_position.side = Side::Buy;
//                     input_position.quantity = 1.0;
//                     input_position.enter_fees_total = 3.0;
//                     input_position.enter_value_gross = 100.0;
//                     input_position
//                 })
//             })
//         });
//         mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
//         mock_repository.set_statistics = Some(|_, _| Ok(()));
//         mock_repository.set_exited_position = Some(|_, _| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::CloseLong;
//         input_fill.quantity = -1.0;
//         input_fill.fill_value_gross = 200.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let updated_cash = updated_repository.balance.unwrap().available;
//         let updated_value = updated_repository.balance.unwrap().total;

//         assert!(result.is_ok());
//         // LONG result_profit_loss = exit_value_gross - enter_value_gross - total_fees
//         // cash += enter_value_gross + result_profit_loss + enter_fees_total
//         assert_eq!(updated_cash, 97.0 + 100.0 + (200.0 - 100.0 - 6.0) + 3.0);
//         // value += result_profit_loss
//         assert_eq!(updated_value, 200.0 + (200.0 - 100.0 - 6.0));
//     }

//     #[test]
//     fn update_from_fill_exiting_long_position_in_loss() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 97.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| {
//             Ok({
//                 Some({
//                     let mut input_position = position();
//                     input_position.side = Side::Buy;
//                     input_position.quantity = 1.0;
//                     input_position.enter_fees_total = 3.0;
//                     input_position.enter_value_gross = 100.0;
//                     input_position
//                 })
//             })
//         });
//         mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
//         mock_repository.set_statistics = Some(|_, _| Ok(()));
//         mock_repository.set_exited_position = Some(|_, _| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::CloseLong;
//         input_fill.quantity = -1.0;
//         input_fill.fill_value_gross = 50.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let updated_cash = updated_repository.balance.unwrap().available;
//         let updated_value = updated_repository.balance.unwrap().total;

//         assert!(result.is_ok());
//         // LONG result_profit_loss = exit_value_gross - enter_value_gross - total_fees
//         // cash += enter_value_gross + result_profit_loss + enter_fees_total
//         assert_eq!(updated_cash, 97.0 + 100.0 + (50.0 - 100.0 - 6.0) + 3.0);
//         // value += result_profit_loss
//         assert_eq!(updated_value, 200.0 + (50.0 - 100.0 - 6.0));
//     }

//     #[test]
//     fn update_from_fill_exiting_short_position_in_profit() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 97.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| {
//             Ok({
//                 Some({
//                     let mut input_position = position();
//                     input_position.side = Side::Sell;
//                     input_position.quantity = -1.0;
//                     input_position.enter_fees_total = 3.0;
//                     input_position.enter_value_gross = 100.0;
//                     input_position
//                 })
//             })
//         });
//         mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
//         mock_repository.set_statistics = Some(|_, _| Ok(()));
//         mock_repository.set_exited_position = Some(|_, _| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::CloseShort;
//         input_fill.quantity = 1.0;
//         input_fill.fill_value_gross = 50.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let updated_cash = updated_repository.balance.unwrap().available;
//         let updated_value = updated_repository.balance.unwrap().total;

//         assert!(result.is_ok());
//         // SHORT result_profit_loss = enter_value_gross - exit_value_gross - total_fees
//         // cash += enter_value_gross + result_profit_loss + enter_fees_total
//         assert_eq!(updated_cash, 97.0 + 100.0 + (100.0 - 50.0 - 6.0) + 3.0);
//         // value += result_profit_loss
//         assert_eq!(updated_value, 200.0 + (100.0 - 50.0 - 6.0));
//     }

//     #[test]
//     fn update_from_fill_exiting_short_position_in_loss() {
//         // Build Portfolio
//         let mut mock_repository = MockRepository::<PnLReturnSummary>::default();
//         mock_repository.get_balance = Some(|_| {
//             Ok(Balance {
//                 time: Utc::now(),
//                 total: 200.0,
//                 available: 97.0,
//             })
//         });
//         mock_repository.remove_position = Some(|_| {
//             Ok({
//                 Some({
//                     let mut input_position = position();
//                     input_position.side = Side::Sell;
//                     input_position.quantity = -1.0;
//                     input_position.enter_fees_total = 3.0;
//                     input_position.enter_value_gross = 100.0;
//                     input_position
//                 })
//             })
//         });
//         mock_repository.get_statistics = Some(|_| Ok(PnLReturnSummary::default()));
//         mock_repository.set_statistics = Some(|_, _| Ok(()));
//         mock_repository.set_exited_position = Some(|_, _| Ok(()));
//         mock_repository.set_balance = Some(|_, _| Ok(()));
//         let mut portfolio = new_mocked_portfolio(mock_repository).unwrap();

//         // Input FillEvent
//         let mut input_fill = fill_event();
//         input_fill.decision = Decision::CloseShort;
//         input_fill.quantity = 1.0;
//         input_fill.fill_value_gross = 150.0;
//         input_fill.fees = Fees {
//             exchange: 1.0,
//             slippage: 1.0,
//             network: 1.0,
//         };

//         let result = portfolio.update_from_fill(&input_fill);
//         let updated_repository = portfolio.repository;
//         let updated_cash = updated_repository.balance.unwrap().available;
//         let updated_value = updated_repository.balance.unwrap().total;

//         assert!(result.is_ok());
//         // SHORT result_profit_loss = enter_value_gross - exit_value_gross - total_fees
//         // cash += enter_value_gross + result_profit_loss + enter_fees_total
//         assert_eq!(updated_cash, 97.0 + 100.0 + (100.0 - 150.0 - 6.0) + 3.0);
//         // value += result_profit_loss
//         assert_eq!(updated_value, 200.0 + (100.0 - 150.0 - 6.0));
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_close_long() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Buy;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));
//         signals.insert(Decision::Short, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::CloseLong);
//     }

//     #[test]
//     fn parse_signal_decisions_to_none_with_some_long_position_and_long_signal() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Buy;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::Long, SignalStrength(1.0));
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert!(actual.is_none())
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_close_long_with_conflicting_signals() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Buy;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));
//         signals.insert(Decision::Short, SignalStrength(1.0));
//         signals.insert(Decision::Long, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::CloseLong);
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_close_short() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Sell;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));
//         signals.insert(Decision::Long, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::CloseShort);
//     }

//     #[test]
//     fn parse_signal_decisions_to_none_with_some_short_position_and_short_signal() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Sell;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));
//         signals.insert(Decision::Short, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert!(actual.is_none())
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_close_short_with_conflicting_signals() {
//         // Some(Position)
//         let mut position = position();
//         position.side = Side::Sell;
//         let position = Some(position);
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));
//         signals.insert(Decision::Short, SignalStrength(1.0));
//         signals.insert(Decision::Long, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::CloseShort);
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_long() {
//         let position = None;
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::Long, SignalStrength(1.0));
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::Long);
//     }

//     #[test]
//     fn parse_signal_decisions_to_net_short() {
//         let position = None;
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::Short, SignalStrength(1.0));
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual.unwrap().0, &Decision::Short);
//     }

//     #[test]
//     fn parse_signal_decisions_to_none_with_conflicting_signals() {
//         let position = None;
//         let position = position.as_ref();

//         // Signals HashMap
//         let mut signals = HashMap::with_capacity(4);
//         signals.insert(Decision::Long, SignalStrength(1.0));
//         signals.insert(Decision::CloseShort, SignalStrength(1.0));
//         signals.insert(Decision::Short, SignalStrength(1.0));
//         signals.insert(Decision::CloseLong, SignalStrength(1.0));

//         let actual = parse_signal_decisions(&position, &signals);

//         assert_eq!(actual, None);
//     }
// }
