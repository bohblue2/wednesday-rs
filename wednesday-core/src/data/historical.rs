use crate::model::enums::Feed;

use super::FeedGenerator;

pub struct HistoricalMarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    pub iterator: Iter,
}

impl<Iter, Event> FeedGenerator<Event> for HistoricalMarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    fn next(&mut self) -> Feed<Event> {
        self.iterator
            .next()
            .map_or(Feed::Finished, Feed::Next)
    }
}

impl<Iter, Event> HistoricalMarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    pub fn new<IntoIter>(iterator: IntoIter) -> Self 
    where
        IntoIter: IntoIterator<IntoIter = Iter, Item = Event>,
    {
        Self {
            iterator: iterator.into_iter(),
        }
    }
}