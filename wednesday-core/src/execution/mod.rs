pub mod simulated;

use crate::model::{execution_error::ExecutionError, fill_event::FillEvent, order_event::OrderEvent};

pub trait ExecutionClient {
    fn generate_fill(&self, order: &OrderEvent) -> Result<FillEvent, ExecutionError>;
}
