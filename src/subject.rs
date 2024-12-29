pub(crate) mod behavior_subject;
pub(crate) mod publish_subject;

use std::rc::Rc;

use crate::stream::stream_defer::DeferStream;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> Rc<DeferStream<Self::Item>>;
    fn close(&mut self);
    fn push(&mut self, value: Self::Item);
}
