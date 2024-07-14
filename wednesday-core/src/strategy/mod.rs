pub mod sample;
pub mod tick_str1;

use wednesday_model::{events::{DataKind, MarketEvent}, identifiers::{Exchange, Market}, instruments::Instrument};
use crate::model::signal::Signal;

pub trait SignalGenerator {
    fn generate_signal(&mut self, market: &MarketEvent<DataKind>) -> Option<Signal>;
}
