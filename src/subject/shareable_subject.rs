use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::Fuse, Stream, StreamExt};

use crate::Observable;

use super::Subject;

pub(crate) struct ShareableSubject<S: Stream, Sub: Subject<Item = S::Item>> {
    stream: Pin<Box<Fuse<S>>>,
    subject: Sub,
}

impl<S: Stream, Sub: Subject<Item = S::Item>> ShareableSubject<S, Sub> {
    pub(crate) fn new(stream: S, subject: Sub) -> Self {
        Self {
            stream: Box::pin(stream.fuse()),
            subject,
        }
    }

    pub(crate) fn subscribe(&mut self) -> Observable<S::Item> {
        self.subject.subscribe()
    }

    pub(crate) fn poll_next(&mut self, cx: &mut Context<'_>) {
        match self.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(value)) => self.subject.next(value),
            Poll::Ready(None) => self.subject.close(),
            Poll::Pending => {}
        }
    }
}
