use std::rc::Rc;

use futures::Stream;

use super::{defer::DeferStream, event::Event};

pub struct ConsumableStream<T> {
    inner: Rc<DeferStream<T>>,
}

impl<T> ConsumableStream<T> {
    pub(crate) fn new(inner: Rc<DeferStream<T>>) -> Self {
        Self { inner }
    }

    pub fn into_stream(self) -> Option<impl Stream<Item = Event<T>>> {
        Rc::into_inner(self.inner)
    }
}
