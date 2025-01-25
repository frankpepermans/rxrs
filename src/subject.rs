pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;
pub mod shareable_subject;

use std::sync::{Arc, RwLock};

use crate::{Controller, Event, Observable};

type Subscription<T> = Arc<RwLock<Controller<Event<T>>>>;

pub trait Subject {
    type Item;

    fn subscribe(&mut self) -> Observable<Self::Item>;
    fn close(&mut self);
    fn next(&mut self, value: Self::Item);
    fn for_each_subscription<F: FnMut(&mut Subscription<Self::Item>)>(&mut self, f: F);
}
