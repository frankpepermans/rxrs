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

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.write().unwrap().is_done = true;
        }
    }

    fn next(&mut self, value: Self::Item) {
        let rc = Arc::new(value);

        if let ReplayStrategy::BufferSize(size) = &self.replay_strategy {
            if self.buffer.len() == *size {
                self.buffer.pop_front();
            }
        }

        self.buffer.push_back(Arc::clone(&rc));

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.write().unwrap().push(Event(Arc::clone(&rc)));
        }
    }

    fn for_each_subscription<F: FnMut(&mut super::Subscription<Self::Item>)>(&mut self, mut f: F) {
        for mut sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            f(&mut sub);
        }
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
