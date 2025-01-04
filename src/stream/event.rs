use std::{
    ops::Deref,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use crate::Controller;

#[derive(Debug)]
pub struct Event<T>(pub(crate) Rc<T>);

impl<T> Event<T> {
    pub fn borrow_value(&self) -> &T {
        &self.0
    }

    pub fn try_unwrap(self) -> Result<T, Rc<T>> {
        Rc::try_unwrap(self.0)
    }
}

impl<T: Clone> Event<T> {
    pub fn unwrap(self) -> T {
        Rc::unwrap_or_clone(self.0)
    }
}

impl<T> Deref for Event<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.borrow_value()
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: PartialEq> PartialEq for Event<T> {
    fn eq(&self, other: &Self) -> bool {
        self.borrow_value() == other.borrow_value()
    }
}

impl<T> From<T> for Event<T> {
    fn from(value: T) -> Self {
        Event(value.into())
    }
}
pub struct EventStream<T, S: Stream<Item = T>> {
    stream: Pin<Box<S>>,
    controller: Controller<Event<T>>,
}

impl<T, S: Stream<Item = T>> EventStream<T, S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: Box::pin(stream),
            controller: Controller::new(),
        }
    }
}

impl<T, S: Stream<Item = T>> Stream for EventStream<T, S> {
    type Item = Event<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        match this.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(it)) => {
                let event = Rc::new(it);
                this.controller.push(Event(Rc::clone(&event)));
            }
            Poll::Ready(None) => {
                this.controller.is_done = true;
            }
            _ => {}
        };

        this.controller.pop()
    }
}
