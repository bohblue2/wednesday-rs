use tokio_stream::StreamExt;
use tracing::{debug, info};
use wednesday_connector::{exchange::bybit::linear::BybitPerpetualsUsd, stream::Streams, subscriber::subscription::kind::PublicTrades};
use wednesday_model::instruments::InstrumentKind;

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

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    init_logging();
    // Install a default CryptoProvider
    rustls::crypto::ring::default_provider().install_default().unwrap();
    let streams = Streams::<PublicTrades>::builder()
        // .subscribe([
        //     (BinanceSpot::default(), "pepe", "usdt", InstrumentKind::CryptoSpot, OrderBooksL2)])
        .subscribe([
            (BybitPerpetualsUsd::default(), "btc", "usdt", InstrumentKind::CryptoPerpetual, PublicTrades)
        ])
        .init()
        .await
        .unwrap();

    let mut joined = streams.join_map().await;

    while let Some((exchange, ob)) = joined.next().await {
        info!(%exchange, ?ob, "Received orderbook");
    }

} 