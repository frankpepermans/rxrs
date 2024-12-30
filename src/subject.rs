pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;

use crate::stream::observable::Observable;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> Observable<Self::Item>;
    fn close(&mut self);
    fn push(&mut self, value: Self::Item);
}
