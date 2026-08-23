use std::{
    collections::VecDeque,
    task::{Context, Poll, Waker},
};

#[derive(Clone)]
pub struct Controller<T> {
    buffer: VecDeque<T>,
    /// Only ever assigned directly when seeding a fresh `Controller` for a
    /// subscription that joins an already closed `Subject` - at that point
    /// there is no subscriber to wake yet. Every later transition to done
    /// goes through [`Controller::close`].
    pub(crate) is_done: bool,
    /// The waker of the single `Observable` reading from this controller, if
    /// it is currently parked on an empty buffer.
    waker: Option<Waker>,
}

impl<T> Controller<T> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            is_done: false,
            waker: None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn push(&mut self, value: T) {
        self.buffer.push_back(value);

        self.wake();
    }

    pub(crate) fn close(&mut self) {
        self.is_done = true;

        self.wake();
    }

    pub(crate) fn pop(&mut self, cx: &Context<'_>) -> Poll<Option<T>> {
        match self.buffer.pop_front() {
            Some(it) => Poll::Ready(Some(it)),
            None => {
                if self.is_done {
                    Poll::Ready(None)
                } else {
                    self.register(cx.waker());

                    Poll::Pending
                }
            }
        }
    }

    fn register(&mut self, waker: &Waker) {
        match &self.waker {
            Some(it) if it.will_wake(waker) => {}
            _ => self.waker = Some(waker.clone()),
        }
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}
