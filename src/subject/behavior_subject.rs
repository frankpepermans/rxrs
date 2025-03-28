use std::sync::{Arc, RwLock, Weak};

use crate::{Controller, Event, Observable};

use super::Subject;

type Subscription<T> = Weak<RwLock<Controller<Event<T>>>>;

pub struct BehaviorSubject<T> {
    subscriptions: Vec<Subscription<T>>,
    is_closed: bool,
    value: Arc<T>,
}

impl<T> Subject for BehaviorSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> Observable<Self::Item> {
        let mut stream = Controller::new();

        stream.is_done = self.is_closed;

        let stream = Arc::new(RwLock::new(stream));

        self.subscriptions.push(Arc::downgrade(&stream));

        stream.write().unwrap().push(Event(Arc::clone(&self.value)));

        Observable::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        self.for_each_subscription(|it| {
            it.write().unwrap().is_done = true;
        });
    }

    fn next(&mut self, value: Self::Item) {
        let rc = Arc::new(value);

        self.value = Arc::clone(&rc);

        self.for_each_subscription(|it| {
            it.write().unwrap().push(Event(Arc::clone(&rc)));
        });
    }

    fn for_each_subscription<F: FnMut(&mut super::Subscription<Self::Item>)>(&mut self, mut f: F) {
        self.subscriptions.retain(|sub| {
            sub.upgrade().is_some_and(|mut it| {
                f(&mut it);

                true
            })
        });
    }
}

#[allow(clippy::new_without_default)]
impl<T> BehaviorSubject<T> {
    pub fn new(value: T) -> Self {
        Self {
            subscriptions: Vec::new(),
            is_closed: false,
            value: value.into(),
        }
    }

    pub fn get_value(&self) -> &T {
        &self.value
    }
}

impl<T> Drop for BehaviorSubject<T> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, StreamExt};

    use crate::{BehaviorSubject, PublishSubject, Subject};

    #[test]
    fn can_subscribe_multiple_times() {
        block_on(async {
            let mut subject = BehaviorSubject::new(0);

            assert_eq!(subject.get_value(), &0);

            let (stream_a, stream_b) = (subject.subscribe(), subject.subscribe());

            subject.next(1);
            subject.close();

            let events_a = stream_a.map(|it| *it).collect::<Vec<_>>().await;
            let events_b = stream_b.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events_a, [0, 1]);
            assert_eq!(events_b, [0, 1]);
        });
    }

    #[test]
    fn replays_latest_event() {
        block_on(async {
            let mut subject_a = PublishSubject::new();
            let mut subject_b = BehaviorSubject::new(0);

            subject_a.next(1);
            subject_b.next(1);
            subject_a.close();
            subject_b.close();

            let (stream_a, stream_b) = (subject_a.subscribe(), subject_b.subscribe());
            let events_a = stream_a.map(|it| *it).collect::<Vec<_>>().await;
            let events_b = stream_b.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events_a, []);
            assert_eq!(events_b, [1]);
        });
    }

    #[test]
    fn can_get_value() {
        let mut subject = BehaviorSubject::new(0);

        assert_eq!(subject.get_value(), &0);

        subject.next(1);

        assert_eq!(subject.get_value(), &1);
    }
}
