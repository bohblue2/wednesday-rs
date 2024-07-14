use crate::enums::{OrderSide, OrderType};

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub instrument_id: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: f64,
    pub quantity: f64,
    pub ts_event: f64,
    pub ts_init: f64,
}

impl Order {
    pub fn new(
        instrument_id: String,
        side: OrderSide,
        order_type: OrderType,
        price: f64,
        quantity: f64,
        ts_event: f64,
        ts_init: f64,
    ) -> Self {
        Order {
            instrument_id,
            side,
            order_type,
            price,
            quantity,
            ts_event,
            ts_init,
        }
    }

    pub fn default() -> Self {
        Order {
            instrument_id: "".to_string(),
            side: OrderSide::None,
            order_type: OrderType::None,
            price: 0.0,
            quantity: 0.0,
            ts_event: 0.0,
            ts_init: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderEnum {
    LimitOrder(Order),
    CancelOrder(Order),
}
