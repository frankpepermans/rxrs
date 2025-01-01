use std::{
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use super::controller::Controller;

#[derive(Debug)]
pub struct Event<T>(pub(crate) Rc<T>);

impl<T> Event<T> {
    pub fn as_inner_ref(&self) -> &T {
        &self.0
    }

    pub fn try_unwrap(self) -> Result<T, Rc<T>> {
        Rc::try_unwrap(self.0)
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
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

    pub(crate) fn is_done(&self) -> bool {
        self.controller.is_done
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
