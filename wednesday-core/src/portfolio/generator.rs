use crate::model::{
    order_event::OrderEvent,
    portfolio_error::PortfolioError,
    signal::{Signal, SignalForceExit},
};

pub trait OrderGenerator {
    fn generate_order(&mut self, signal: &Signal) -> Result<Option<OrderEvent>, PortfolioError>;

    fn generate_exit_order(&mut self, signal: &SignalForceExit) -> Result<Option<OrderEvent>, PortfolioError>;
}
