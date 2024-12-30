use std::{
    cell::RefCell,
    pin::Pin,
    task::{Context, Poll},
};

use controller::StreamController;
use futures::Stream;

use crate::prelude::*;

pub(crate) struct DeferStream<T> {
    pub(crate) inner: RefCell<StreamController<Event<T>>>,
}

impl<T> DeferStream<T> {
    pub(crate) fn new(inner: RefCell<StreamController<Event<T>>>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for DeferStream<T> {
    type Item = Event<T>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.borrow_mut().next()
    }
}
