use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    prelude::Event,
    stream::{controller::Controller, observable::Observable},
};

use super::Subject;

pub struct PublishSubject<T> {
    subscriptions: Vec<Weak<RefCell<Controller<Event<T>>>>>,
    is_closed: bool,
}

impl<T> Subject for PublishSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> Observable<Self::Item> {
        let mut stream = Controller::new();

        stream.is_done = self.is_closed;

        let stream = Rc::new(RefCell::new(stream));

        self.subscriptions.push(Rc::downgrade(&stream));

        Observable::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().is_done = true;
        }
    }

    fn push(&mut self, value: Self::Item) {
        let rc = Rc::new(value);

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().push(Event(rc.clone()));
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
