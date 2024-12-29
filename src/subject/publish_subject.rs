use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::stream::{controller::StreamController, defer::DeferStream};

use super::Subject;

pub struct PublishSubject<T> {
    subscriptions: Vec<Weak<DeferStream<T>>>,
    is_closed: bool,
}

impl<T: Clone + Unpin> Subject for PublishSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> DeferStream<Self::Item> {
        let mut stream = StreamController::new();

        stream.is_done = self.is_closed;

        let stream = Rc::new(DeferStream::new(RefCell::new(stream)));

        self.subscriptions.push(Rc::downgrade(&stream));

        <DeferStream<Self::Item> as Clone>::clone(&stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.inner.borrow_mut().is_done = true;
        }
    }

    fn push(&mut self, value: Self::Item) {
        let rc = Rc::new(value);

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.inner.borrow_mut().push(rc.clone());
        }
    }
}

impl<T> PublishSubject<T> {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            is_closed: false,
        }
    }
}
