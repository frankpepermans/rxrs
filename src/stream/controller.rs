use std::{collections::VecDeque, rc::Rc, task::Poll};

#[derive(Clone)]
pub struct StreamController<T> {
    buffer: VecDeque<Rc<T>>,
    pub(crate) is_done: bool,
}

impl<T: Clone> StreamController<T> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            is_done: false,
        }
    }

    pub(crate) fn push(&mut self, value: Rc<T>) {
        self.buffer.push_back(value);
    }

    pub(crate) fn next(&mut self) -> Poll<Option<T>> {
        match self.buffer.pop_front() {
            Some(it) => Poll::Ready(Some(it.as_ref().to_owned())),
            None => {
                if self.is_done {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}
