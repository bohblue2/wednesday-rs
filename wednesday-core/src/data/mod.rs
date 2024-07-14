pub mod historical;
pub mod live;

use crate::model::enums::Feed;

pub trait FeedGenerator<Event> {
    fn next(&mut self) -> Feed<Event>;
}