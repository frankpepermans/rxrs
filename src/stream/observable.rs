use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{stream::FusedStream, Stream};

use crate::{Controller, Event};

pub struct Observable<T> {
    inner: Arc<RwLock<Controller<Event<T>>>>,
}

impl<T> Observable<T> {
    pub(crate) fn new(inner: Arc<RwLock<Controller<Event<T>>>>) -> Self {
        Self { inner }
    }
}

impl<T> FusedStream for Observable<T> {
    fn is_terminated(&self) -> bool {
        self.inner.read().unwrap().is_done
    }
}

impl<T> Stream for Observable<T> {
    type Item = Event<T>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().inner.write().unwrap().pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let inner = self.inner.read().unwrap();
        let lower_bound = inner.len();
        let upper_bound = if inner.is_done {
            Some(lower_bound)
        } else {
            None
        };

        (lower_bound, upper_bound)
    }
}
