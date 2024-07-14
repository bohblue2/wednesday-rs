pub mod in_memory;

use uuid::Uuid;
use wednesday_model::identifiers::{Market, MarketId};

use crate::model::{balance::Balance, position::{Position, PositionId}, repository_error::RepositoryError};


pub trait PositionHandler {
    fn set_open_position(
        &mut self,
        position: Position
    ) -> Result<(), RepositoryError>;
    fn get_open_position(
        &mut self,
        position_id: &PositionId
    ) -> Result<Option<Position>, RepositoryError>; 

    // NOTE: engine_id 를 argument 로 받아야 하는지 고민해보자, repository 에서 engine_id 를 소유하도록 변경
    fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
        &mut self,
        engine_id: Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError>;

    fn remove_position(
        &mut self,
        position_id: &PositionId
    ) -> Result<Option<Position>, RepositoryError>;   

    fn set_exited_position(
        &mut self,
        engine_id: Uuid,
        position: Position
    ) -> Result<(), RepositoryError>;

    fn get_exited_positions(
        &mut self,
        engine_id: Uuid
    ) -> Result<Vec<Position>, RepositoryError>; 
}

pub trait BalanceHandler {
    fn set_balance(&mut self, engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError>;
    fn get_balance(&mut self, engine_id: Uuid) -> Result<Balance, RepositoryError>;
}

pub trait StatisticHandler<Statistic> {
    fn set_statistics(
        &mut self, 
        market_id: MarketId,
        statistic: Statistic
    ) -> Result<(), RepositoryError>;
    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError>;
}

// NOTE: We need refactor this code, move to poition.rs
pub type ExitedPositionsId = String;

pub fn determine_exited_positions_id(engine_id: Uuid) -> ExitedPositionsId {
    format!("{}-exited-positions", engine_id)
}