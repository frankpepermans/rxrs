use std::{
    collections::VecDeque,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures::Stream;

#[derive(Clone)]
pub struct StreamController<T> {
    buffer: VecDeque<Rc<T>>,
    pub(crate) is_done: bool,
}

impl<T> StreamController<T> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            is_done: false,
        }
    }

    pub(crate) fn push(&mut self, value: Rc<T>) {
        self.buffer.push_back(value);
    }
}

impl<T: Unpin> Stream for StreamController<T> {
    type Item = Rc<T>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        match this.buffer.pop_front() {
            Some(it) => Poll::Ready(Some(it)),
            None => {
                if this.is_done {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}
