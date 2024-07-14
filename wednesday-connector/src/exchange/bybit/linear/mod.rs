
use wednesday_model::identifiers::ExchangeId;

use super::{Bybit, ExchangeServer};

/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WS_BASE_URL_BYBIT_PERPETUALS_USD: &str = "wss://stream.bybit.com/v5/public/linear";

pub type BybitPerpetualsUsd = Bybit<BybitServerPerpetualsUsd>;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BybitServerPerpetualsUsd;

impl ExchangeServer for BybitServerPerpetualsUsd {
    const ID: ExchangeId = ExchangeId::BybitPerpetualsUsd;

    fn ws_url() -> &'static str {
        WS_BASE_URL_BYBIT_PERPETUALS_USD
    }
}
