use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    prelude::Event,
    stream::{consumable::ConsumableStream, controller::StreamController, defer::DeferStream},
};

use super::Subject;

pub struct BehaviorSubject<T> {
    subscriptions: Vec<Weak<DeferStream<T>>>,
    is_closed: bool,
    latest_event: Option<Rc<T>>,
}

impl<T> Subject for BehaviorSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> ConsumableStream<Self::Item> {
        let mut stream = StreamController::new();

        stream.is_done = self.is_closed;

        let stream = Rc::new(DeferStream::new(RefCell::new(stream)));

        self.subscriptions.push(Rc::downgrade(&stream));

        if let Some(event) = &self.latest_event {
            stream.inner.borrow_mut().push(Event(Rc::clone(&event)));
        }

        ConsumableStream::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.inner.borrow_mut().is_done = true;
        }
    }

    fn push(&mut self, value: Self::Item) {
        let rc = Rc::new(value);

        self.latest_event = Some(Rc::clone(&rc));

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.inner.borrow_mut().push(Event(Rc::clone(&rc)));
        }
    }
}

impl<T> BehaviorSubject<T> {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            is_closed: false,
            latest_event: None,
        }
    }
}
