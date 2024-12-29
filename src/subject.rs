pub mod behavior_subject;
pub mod publish_subject;

use futures::Stream;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> impl Stream<Item = Self::Item>;
    fn close(&mut self);
    fn push(&mut self, value: Self::Item);
}
