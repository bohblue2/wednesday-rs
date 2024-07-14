
use wednesday_model::identifiers::ExchangeId;

use crate::exchange::connector::ExchangeServer;

use super::Bybit;

pub const WS_BASE_URL_BYBIT_SPOT: &str = "wss://stream.bybit.com/v5/public/spot";

pub type BybitSpot = Bybit<BybitServerSpot>;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Hash)]
pub struct BybitServerSpot;

impl ExchangeServer for BybitServerSpot {
    const ID: ExchangeId = ExchangeId::BybitSpot;
    fn ws_url() -> &'static str {
        WS_BASE_URL_BYBIT_SPOT
    }
}