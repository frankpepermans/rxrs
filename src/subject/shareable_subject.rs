use std::{
    rc::Rc,
    task::{Context, Poll},
};

use futures::{stream::Fuse, Stream, StreamExt};

use crate::{Event, Observable};

use super::Subject;

pub(crate) struct ShareableSubject<S: Stream, Sub: Subject<Item = S::Item>> {
    stream: Fuse<S>,
    subject: Sub,
}

impl<S: Stream + Unpin, Sub: Subject<Item = S::Item>> ShareableSubject<S, Sub> {
    pub(crate) fn new(stream: S, subject: Sub) -> Self {
        Self {
            stream: stream.fuse(),
            subject,
        }
    }

    pub(crate) fn subscribe(&mut self) -> Observable<S::Item> {
        self.subject.subscribe()
    }

    pub(crate) fn poll_next(&mut self, cx: &mut Context<'_>) {
        match self.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(value)) => {
                let rc = Rc::new(value);

                self.subject
                    .for_each_subscription(|sub| sub.borrow_mut().push(Event(rc.clone())));
            }
            Poll::Ready(None) => {
                self.subject
                    .for_each_subscription(|sub| sub.borrow_mut().is_done = true);
            }
            Poll::Pending => {}
        }
    }
}
