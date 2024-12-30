use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use controller::StreamController;
use futures::Stream;

use crate::prelude::*;

pub struct DeferStream<T> {
    pub(crate) inner: RefCell<StreamController<Event<T>>>,
}

impl<T> DeferStream<T> {
    pub(crate) fn new(inner: RefCell<StreamController<Event<T>>>) -> Self {
        Self { inner }
    }

    pub fn consume(self: Rc<Self>) -> Option<Self> {
        Rc::into_inner(self)
    }
}

impl<T> Stream for DeferStream<T> {
    type Item = Event<T>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.borrow_mut().next()
    }
}
