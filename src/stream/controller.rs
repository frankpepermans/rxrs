use std::{collections::VecDeque, task::Poll};

#[derive(Clone)]
pub struct Controller<T> {
    buffer: VecDeque<T>,
    pub(crate) is_done: bool,
}

impl<T> Controller<T> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            is_done: false,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn push(&mut self, value: T) {
        self.buffer.push_back(value);
    }

    pub(crate) fn pop(&mut self) -> Poll<Option<T>> {
        match self.buffer.pop_front() {
            Some(it) => Poll::Ready(Some(it)),
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
