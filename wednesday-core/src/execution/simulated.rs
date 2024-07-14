use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::model::{fee::Fees, fill_event::FillEvent};

use super::ExecutionClient;


pub struct SimExecConfig {
    /// Simulated Fee percentage to be used for each ['Fees'] field in decimal from (eg/ 0.01 for 1%)
    pub simulated_fees_pct: Fees
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct SimulatedExecution {
    fees_pct: Fees,
}

impl ExecutionClient for SimulatedExecution {
    fn generate_fill(
        &self, 
        order: &crate::model::order_event::OrderEvent
    ) -> Result<crate::model::fill_event::FillEvent, crate::model::execution_error::ExecutionError> {
        let fill_value_gross = SimulatedExecution::calculate_fill_value_gross(order); 

        Ok(FillEvent {
            // NOTE: timestamp 에서 local_ts, exchange_ts 를 쓰는게 맞지 않나 ?
            timestamp: Utc::now(),
            exchange: order.exchange.clone(),
            instrument: order.instrument.clone(),
            market_meta: order.market_meta,
            decision: order.decision,
            quantity: order.quantity,
            fill_value_gross,
            fees: self.calculate_fees(&fill_value_gross),
        })
    }
}

impl SimulatedExecution {
    pub fn new(config: SimExecConfig) -> Self {
        Self {
            fees_pct: config.simulated_fees_pct
        }
    }
    
    fn calculate_fill_value_gross(order: &crate::model::order_event::OrderEvent) -> f64 {
        // NOTE: 이부분 jarvis 랑 비슷한 패턴이내. 오더가 가격정보 실시간 물고있는거
        order.quantity.abs() * order.market_meta.close
    }

    fn calculate_fees(&self, fill_value_gross: &f64) -> Fees {
        Fees {
            exchange: self.fees_pct.exchange * fill_value_gross,
            slippage: self.fees_pct.slippage * fill_value_gross,
            // network: self.fees_pct.network * fill_value_gross,
        }
    } 
}

// NOTE: Commented in purpose.
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::test_util::order_event;

//     #[test]
//     fn should_generate_ok_fill_event_with_valid_order_event_provided() {
//         let simulated_execution = SimulatedExecution::new(Config {
//             simulated_fees_pct: Fees {
//                 exchange: 0.1,
//                 slippage: 0.05,
//                 network: 0.0,
//             },
//         });

//         let mut input_order = order_event();
//         input_order.quantity = 10.0;
//         input_order.market_meta.close = 10.0;

//         let actual_result = simulated_execution.generate_fill(&input_order);

//         let expected_fill_value_gross = 100.0;
//         let expected_fees = Fees {
//             exchange: 10.0,
//             slippage: 5.0,
//             network: 0.0,
//         };

//         assert!(actual_result.is_ok());
//         let actual_result = actual_result.unwrap();
//         assert_eq!(actual_result.fill_value_gross, expected_fill_value_gross);
//         assert_eq!(actual_result.fees, expected_fees);
//     }

//     #[test]
//     fn should_calculate_fill_value_gross_correctly() {
//         let mut input_order = order_event();
//         input_order.quantity = 100.0;
//         input_order.market_meta.close = 10.0;

//         let actual = SimulatedExecution::calculate_fill_value_gross(&input_order);

//         let expected = 100.0 * 10.0;

//         assert_eq!(actual, expected)
//     }

//     #[test]
//     fn should_calculate_fill_value_gross_correctly_with_negative_order_quantity_provided() {
//         let mut input_order = order_event();
//         input_order.quantity = -(100.0);
//         input_order.market_meta.close = 10.0;

//         let actual = SimulatedExecution::calculate_fill_value_gross(&input_order);

//         let expected = (100.0 * 10.0) as f64;

//         assert_eq!(actual, expected)
//     }

//     #[test]
//     fn should_calculate_simulated_fees_correctly() {
//         let simulated_execution = SimulatedExecution::new(Config {
//             simulated_fees_pct: Fees {
//                 exchange: 0.5,
//                 slippage: 0.1,
//                 network: 0.001,
//             },
//         });

//         let input_fill_value_gross = 100.0;

//         let actual_result = simulated_execution.calculate_fees(&input_fill_value_gross);

//         let expected = Fees {
//             exchange: 50.0,
//             slippage: 10.0,
//             network: 0.1,
//         };

//         assert_eq!(actual_result, expected)
//     }
// }
