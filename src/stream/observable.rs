use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures::Stream;

use crate::{Controller, Event};

pub struct Observable<T> {
    inner: Rc<RefCell<Controller<Event<T>>>>,
}

impl<T> Observable<T> {
    pub(crate) fn new(inner: Rc<RefCell<Controller<Event<T>>>>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for Observable<T> {
    type Item = Event<T>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().inner.borrow_mut().pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.inner.borrow().len();

        (size, Some(size))
    }
}
