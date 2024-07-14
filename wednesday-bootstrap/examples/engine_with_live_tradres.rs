use std::{collections::HashMap, fs, sync::Arc, time::Duration};

use base64::Engine;
use chrono::Utc;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::event;
use uuid::Uuid;
use wednesday_connector::{exchange::binance::spot::BinanceSpot, stream::Streams, subscriber::subscription::kind::PublicTrades};
use wednesday_core::{data::{historical, live}, engine::{trader::Trader, TradingEngine}, execution::simulated::{SimExecConfig, SimulatedExecution}, model::{event::{Event, EventTx}, fee::Fees}, oms::{allocator::DefaultAllocator, evaluator::DefaultRisk}, portfolio::{repository::{self, in_memory::InMemoryRepository}, MetaPortfolio}, statistic::summary::trading::{Config, TradingSummary}, strategy::{tick_str1::{TickReactStrategy, TickReactStrategyConfig}}};
use wednesday_model::{bar::Bar, events::{DataKind, MarketEvent}, identifiers::{Exchange, ExchangeId, Market}, instruments::{Instrument, InstrumentKind}};
use wednesday_core::statistic::summary::Initialiser;

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}

#[tokio::main]
async fn main() {
    init_logging();
    // Create channel to distribute Commands to the Engine & it's Traders (eg/ Command::Terminate)
    let (_command_tx, command_rx) = mpsc::channel(10);

    // Create Event channal to listen to all Engine Events in real-time
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let event_tx = EventTx::new(event_tx);

    // Generate unique identifier to associate an Engine's components
    let engine_id = Uuid::new_v4();

    // Create the Markt(s) to be traded on (1-to-1 relationship with a Trader)
    let market = Market::new("binance", ("btc", "usdt", InstrumentKind::CryptoSpot));

    // Build global shared-state MetaPortfolio (1-to-1 relashionship with an Engine)
    let portfolio = Arc::new(Mutex::new(
        MetaPortfolio::builder()
            .engine_id(engine_id)
            .markets(vec![market.clone()])
            .starting_cash(10_000.0)
            .repository(InMemoryRepository::new())
            .allocation_manager(DefaultAllocator { default_order_value: 100.0 })
            .risk_manager( DefaultRisk {})
            .statistic_config(Config { 
                starting_equity: 10_000.0, 
                trading_days_per_year: 365, 
                risk_free_return: 0.0
            })
            .build_and_init()
            .expect("failed to build & initialise MetaPortfolio")
        )
    );

    let mut traders = Vec::new();

    let (trader_command_tx, trader_command_rx) = mpsc::channel(10);

    let feed_rx = stream_market_event_trades().await;
    
    traders.push(
        Trader::builder()
            .engine_id(engine_id)
            .market(market.clone())
            .command_rx(trader_command_rx)  
            .event_tx(event_tx.clone())
            .portfolio(Arc::clone(&portfolio))
            .data(live::LiveMarketFeed::new(feed_rx))
            .strategy(TickReactStrategy::new(TickReactStrategyConfig { rsi_period: 14}))
            .execution(SimulatedExecution::new( SimExecConfig {
                simulated_fees_pct: Fees {
                    exchange: 0.1,
                    slippage: 0.05,
                }
            }))
            .build()
            .expect("failed to build trader")
    );

    let traders_commands_txs = HashMap::from([(market, trader_command_tx)]);

    let engine = TradingEngine::builder()
        .engine_id(engine_id)
        .command_rx(command_rx)
        .portfolio(portfolio)
        .traders(traders)
        .trader_command_txs(traders_commands_txs)
        .statistics_summary(TradingSummary::init(Config { 
            starting_equity: 10_000.0, 
            trading_days_per_year: 365, 
            risk_free_return: 0.0
        }))
        .build()
        .expect("failed to build TradingEngine");   

    tokio::spawn(listen_to_engine_events(event_rx));

    match tokio::time::timeout(Duration::from_secs(600), engine.run()).await {
        Ok(_) => println!("Engine run completed successfully."),
        Err(e) => println!("Engine run timed out: {:?}", e),
    }
    // engine.run().await;
}

async fn stream_market_event_trades() -> mpsc::UnboundedReceiver<MarketEvent<DataKind>> {
    rustls::crypto::ring::default_provider().install_default().unwrap();
    let mut streams = Streams::<PublicTrades>::builder()
        .subscribe([(
            BinanceSpot::default(),
            "btc",
            "usdt",
            InstrumentKind::CryptoSpot,
            PublicTrades
        )])
        .init()
        .await
        .unwrap();
    
    // NOTE: 왜 여기서 두번 거쳐서 데이터를 전달하는거지 ?
    let mut trade_rx = streams.select(ExchangeId::BinanceSpot).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(trade) = trade_rx.recv().await {
            tx.send(MarketEvent::from(trade));
        }
    });

    rx
}

async fn listen_to_engine_events(mut event_rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = event_rx.recv().await {
        match event {
            Event::Market(market) => {
                // Market Event occurred in Engine
                println!("{market:?}");
            }
            Event::Signal(signal) => {
                // Signal Event occurred in Engine
                println!("{signal:?}");
            }
            Event::SignalForceExit(_) => {
                // SignalForceExit Event occurred in Engine
            }
            Event::OrderNew(new_order) => {
                // OrderNew Event occurred in Engine
                println!("{new_order:?}");
            }
            Event::OrderUpdate => {
                // OrderUpdate Event occurred in Engine
            }
            Event::Fill(fill_event) => {
                // Fill Event occurred in Engine
                println!("{fill_event:?}");
            }
            Event::PositionNew(new_position) => {
                // PositionNew Event occurred in Engine
                println!("{new_position:?}");
            }
            Event::PositionUpdate(updated_position) => {
                // PositionUpdate Event occurred in Engine
                println!("{updated_position:?}");
            }
            Event::PositionExit(exited_position) => {
                // PositionExit Event occurred in Engine
                println!("{exited_position:?}");
            }
            Event::Balance(balance_update) => {
                // Balance update Event occurred in Engine
                println!("{balance_update:?}");
            }
        }
    }
}
    



    
    