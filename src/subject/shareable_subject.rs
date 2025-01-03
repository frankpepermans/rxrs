use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

use crate::{Controller, Event, Observable};

pin_project! {
    pub(crate) struct ShareableSubject<S: Stream> {
        #[pin]
        stream: Option<Fuse<S>>,
        subscriptions: Vec<Weak<RefCell<Controller<Event<S::Item>>>>>,
    }
}

impl<S: Stream + Unpin> ShareableSubject<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: Some(stream.fuse()),
            subscriptions: Vec::new(),
        }
    }

    pub(crate) fn subscribe(&mut self) -> Observable<S::Item> {
        let mut stream = Controller::new();

        stream.is_done = self
            .stream
            .as_ref()
            .map(|it| it.is_terminated())
            .unwrap_or(false);

        let stream = Rc::new(RefCell::new(stream));

        self.subscriptions.push(Rc::downgrade(&stream));

        Observable::new(stream)
    }

    pub(crate) fn poll_next(&mut self, cx: &mut Context<'_>) {
        let stream = self.stream.as_mut().unwrap();

        match stream.poll_next_unpin(cx) {
            Poll::Ready(Some(value)) => {
                let rc = Rc::new(value);

                for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
                    sub.borrow_mut().push(Event(rc.clone()));
                }
            }
            Poll::Ready(None) => {
                for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
                    sub.borrow_mut().is_done = true;
                }
            }
            Poll::Pending => {}
        }
    }
}
