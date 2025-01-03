pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;
pub mod shareable_subject;

use crate::Observable;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> Observable<Self::Item>;
    fn close(&mut self);
    fn next(&mut self, value: Self::Item);
}
