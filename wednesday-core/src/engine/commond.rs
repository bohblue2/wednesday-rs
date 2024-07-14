use tokio::sync::oneshot;
use wednesday_model::identifiers::Market;

use crate::model::{engine_error::EngineError, position::Position};

#[derive(Debug)]
pub enum EngineCommand {
    FetchOpenPositions(oneshot::Sender<Result<Vec<Position>, EngineError>>),
    Terminate(String),
    ExitAllPositions,
    ExitPosition(Market),
}

