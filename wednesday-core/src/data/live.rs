use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError::{Empty, Disconnected};

use crate::model::enums::Feed;

use super::FeedGenerator;


pub struct LiveMarketFeed<Event> {
    pub market_rx: UnboundedReceiver<Event>,
}

impl<Event> FeedGenerator<Event> for LiveMarketFeed<Event> {
    fn next(&mut self) -> Feed<Event> {
        loop {
            match self.market_rx.try_recv() {
                Ok(event) => break Feed::Next(event),
                Err(Empty) => continue,
                Err(Disconnected) => break Feed::Finished,
            }
        }
    }
}

impl<Event> LiveMarketFeed<Event> {
    pub fn new(market_rx: UnboundedReceiver<Event>) -> Self {
        Self {
            market_rx,
        }
    }   
}