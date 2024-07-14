use tokio::sync::mpsc;
use tracing::{event, warn};
use wednesday_model::events::{DataKind, MarketEvent};

use super::{
    balance::Balance,
    fill_event::FillEvent,
    order_event::OrderEvent,
    position::{exiter::PositionExit, updater::PositionUpdate, Position},
    signal::{Signal, SignalForceExit},
};

#[derive(Debug)]
pub enum Event {
    Market(MarketEvent<DataKind>),
    Signal(Signal),
    SignalForceExit(SignalForceExit),
    OrderNew(OrderEvent),
    OrderUpdate,
    Fill(FillEvent),
    PositionNew(Position),
    PositionUpdate(PositionUpdate),
    PositionExit(PositionExit),
    Balance(Balance),
}

#[derive(Debug, Clone)]
pub struct EventTx {
    receiver_dropped: bool,

    event_tx: mpsc::UnboundedSender<Event>,
}

impl EventTx {
    pub fn new(event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            receiver_dropped: false,
            event_tx,
        }
    }
}

pub trait MessageTransmitter<Message> {
    fn send(&mut self, message: Message);

    fn send_many(&mut self, messages: Vec<Message>);
}

impl MessageTransmitter<Event> for EventTx {
    fn send(&mut self, message: Event) {
        if self.receiver_dropped {
            return;
        }

        if self.event_tx.send(message).is_err() {
            warn!(
                action = "setting receiver_dropped = true",
                why = "event receiver dropped",
                "cannnot send events"
            );
            self.receiver_dropped = true
        }
    }

    fn send_many(&mut self, messages: Vec<Event>) {
        if self.receiver_dropped {
            return;
        }

        messages.into_iter().for_each(|message| {
            self.event_tx.send(message);
        })
    }
}
