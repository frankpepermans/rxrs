use std::{
    collections::VecDeque,
    sync::{Arc, RwLock, Weak},
};

use crate::{Controller, Event, Observable};

use super::Subject;

type Subscription<T> = Weak<RwLock<Controller<Event<T>>>>;

pub(crate) enum ReplayStrategy {
    BufferSize(usize),
    Unbounded,
}

pub struct ReplaySubject<T> {
    replay_strategy: ReplayStrategy,
    subscriptions: Vec<Subscription<T>>,
    is_closed: bool,
    buffer: VecDeque<Arc<T>>,
}

impl<T> Subject for ReplaySubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> Observable<Self::Item> {
        let mut stream = Controller::new();

        stream.is_done = self.is_closed;

        let stream = Arc::new(RwLock::new(stream));

        self.subscriptions.push(Arc::downgrade(&stream));

        for event in &self.buffer {
            stream.write().unwrap().push(Event(Arc::clone(event)));
        }

        Observable::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        self.for_each_subscription(|it| {
            it.write().unwrap().close();
        });
    }

    fn next(&mut self, value: Self::Item) {
        let rc = Arc::new(value);

        match &self.replay_strategy {
            ReplayStrategy::BufferSize(size) => {
                if *size > 0 {
                    while self.buffer.len() >= *size {
                        self.buffer.pop_front();
                    }

                    self.buffer.push_back(Arc::clone(&rc));
                }
            }
            ReplayStrategy::Unbounded => self.buffer.push_back(Arc::clone(&rc)),
        }

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
impl<T> ReplaySubject<T> {
    pub fn new() -> Self {
        Self {
            replay_strategy: ReplayStrategy::Unbounded,
            subscriptions: Vec::new(),
            is_closed: false,
            buffer: VecDeque::new(),
        }
    }

    pub fn buffer_size(size: usize) -> Self {
        Self {
            replay_strategy: ReplayStrategy::BufferSize(size),
            subscriptions: Vec::new(),
            is_closed: false,
            buffer: VecDeque::with_capacity(size),
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl<T> Drop for ReplaySubject<T> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, StreamExt};

    use crate::{PublishSubject, ReplaySubject, Subject};

    #[test]
    fn can_subscribe_multiple_times() {
        block_on(async {
            let mut subject = ReplaySubject::new();

            let (stream_a, stream_b) = (subject.subscribe(), subject.subscribe());

            subject.next(1);
            subject.close();

            let events_a = stream_a.map(|it| *it).collect::<Vec<_>>().await;
            let events_b = stream_b.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events_a, [1]);
            assert_eq!(events_b, [1]);
        });
    }

    #[test]
    fn replays_latest_events() {
        block_on(async {
            let mut subject_a = PublishSubject::new();
            let mut subject_b = ReplaySubject::new();

            subject_a.next(1);
            subject_a.next(2);
            subject_a.next(3);
            subject_b.next(1);
            subject_b.next(2);
            subject_b.next(3);
            subject_a.close();
            subject_b.close();

            let (stream_a, stream_b) = (subject_a.subscribe(), subject_b.subscribe());
            let events_a = stream_a.map(|it| *it).collect::<Vec<_>>().await;
            let events_b = stream_b.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events_a, []);
            assert_eq!(events_b, [1, 2, 3]);
        });
    }
}

#[cfg(test)]
mod replay_test {
    use futures::{executor::block_on, StreamExt};

    use crate::{ReplaySubject, Subject};

    #[test]
    fn an_unbounded_subject_replays_every_event() {
        block_on(async {
            let mut subject = ReplaySubject::new();

            for it in 1..=5 {
                subject.next(it);
            }

            let observable = subject.subscribe();

            subject.close();

            assert_eq!(
                observable.map(|it| *it).collect::<Vec<_>>().await,
                [1, 2, 3, 4, 5]
            );
        });
    }

    #[test]
    fn a_bounded_subject_replays_only_the_most_recent_events() {
        block_on(async {
            let mut subject = ReplaySubject::buffer_size(2);

            for it in 1..=5 {
                subject.next(it);
            }

            let observable = subject.subscribe();

            subject.close();

            assert_eq!(observable.map(|it| *it).collect::<Vec<_>>().await, [4, 5]);
        });
    }

    #[test]
    fn buffer_len_never_exceeds_the_buffer_size() {
        let mut subject = ReplaySubject::buffer_size(3);

        assert_eq!(subject.buffer_len(), 0);

        for expected in [1, 2, 3, 3, 3] {
            subject.next(0);

            assert_eq!(subject.buffer_len(), expected);
        }
    }

    #[test]
    fn an_unbounded_buffer_grows_with_every_event() {
        let mut subject = ReplaySubject::new();

        for expected in 1..=5 {
            subject.next(0);

            assert_eq!(subject.buffer_len(), expected);
        }
    }

    #[test]
    fn a_replayed_event_is_also_delivered_live() {
        block_on(async {
            let mut subject = ReplaySubject::new();

            subject.next(1);

            let observable = subject.subscribe();

            subject.next(2);
            subject.close();

            assert_eq!(observable.map(|it| *it).collect::<Vec<_>>().await, [1, 2]);
        });
    }

    #[test]
    fn subscribing_to_a_closed_subject_still_replays_the_buffer() {
        block_on(async {
            let mut subject = ReplaySubject::new();

            subject.next(1);
            subject.next(2);
            subject.close();

            let observable = subject.subscribe();

            assert_eq!(observable.map(|it| *it).collect::<Vec<_>>().await, [1, 2]);
        });
    }

    #[test]
    fn a_buffer_size_of_zero_replays_nothing() {
        block_on(async {
            let mut subject = ReplaySubject::buffer_size(0);

            subject.next(1);

            let observable = subject.subscribe();

            subject.close();

            assert_eq!(observable.map(|it| *it).collect::<Vec<_>>().await, []);
        });
    }

    #[test]
    fn completes_once_its_subject_is_dropped() {
        block_on(async {
            let mut subject = ReplaySubject::new();
            let observable = subject.subscribe();

            subject.next(1);
            drop(subject);

            assert_eq!(observable.map(|it| *it).collect::<Vec<_>>().await, [1]);
        });
    }
}
