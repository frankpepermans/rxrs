use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{Controller, Event, Observable};

use super::Subject;

type Subscription<T> = Weak<RefCell<Controller<Event<T>>>>;

pub struct BehaviorSubject<T> {
    subscriptions: Vec<Subscription<T>>,
    is_closed: bool,
    latest_event: Option<Rc<T>>,
}

impl<T> Subject for BehaviorSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> Observable<Self::Item> {
        let mut stream = Controller::new();

        stream.is_done = self.is_closed;

        let stream = Rc::new(RefCell::new(stream));

        self.subscriptions.push(Rc::downgrade(&stream));

        if let Some(event) = &self.latest_event {
            stream.borrow_mut().push(Event(Rc::clone(event)));
        }

        Observable::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().is_done = true;
        }
    }

    fn next(&mut self, value: Self::Item) {
        let rc = Rc::new(value);

        self.latest_event = Some(Rc::clone(&rc));

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().push(Event(Rc::clone(&rc)));
        }
    }

    fn for_each_subscription<F: FnMut(&mut super::Subscription<Self::Item>)>(&mut self, mut f: F) {
        for mut sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            f(&mut sub);
        }
    }
}

#[allow(clippy::new_without_default)]
impl<T> BehaviorSubject<T> {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            is_closed: false,
            latest_event: None,
        }
    }
}

impl<T> Drop for BehaviorSubject<T> {
    fn drop(&mut self) {
        self.close();
    }
}
