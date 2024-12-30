pub mod behavior_subject;
pub mod publish_subject;

use futures::Stream;

use crate::prelude::Event;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> impl Stream<Item = Event<Self::Item>>;
    fn close(&mut self);
    fn push(&mut self, value: Self::Item);
}
