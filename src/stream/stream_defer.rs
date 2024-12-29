use std::{cell::RefCell, rc::Rc};

use futures::Stream;

use crate::StreamController;

pub struct DeferStream<T> {
    pub(crate) inner: RefCell<StreamController<T>>,
}

impl<T: Clone + Unpin> DeferStream<T> {
    pub(crate) fn new(inner: RefCell<StreamController<T>>) -> Self {
        Self { inner }
    }

    pub fn as_stream(&self) -> impl Stream<Item = Rc<T>> {
        self.inner.borrow().clone()
    }
}
