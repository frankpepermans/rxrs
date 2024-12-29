pub mod behavior_subject;
pub mod publish_subject;

use crate::stream::defer::DeferStream;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> DeferStream<Self::Item>;
    fn close(&mut self);
    fn push(&mut self, value: Self::Item);
}
