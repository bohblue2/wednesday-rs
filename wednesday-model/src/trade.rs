use crate::enums::AggressorSide;

#[derive(Debug, PartialEq, Clone)]
pub struct PublicTrade {
    pub id: String,
    pub price: f64,
    pub quantity: f64,
    pub aggressor_side: AggressorSide,
}
