use std::{
    cell::RefCell,
    pin::Pin,
    task::{Context, Poll},
};

use controller::StreamController;
use futures::Stream;

use crate::prelude::*;

#[derive(Clone)]
pub struct DeferStream<T> {
    pub(crate) inner: RefCell<StreamController<T>>,
}

impl<T: Unpin> DeferStream<T> {
    pub(crate) fn new(inner: RefCell<StreamController<T>>) -> Self {
        Self { inner }
    }
}

impl<T: Clone> Stream for DeferStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.borrow_mut().next()
    }
}
