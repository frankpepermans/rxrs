use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;
use pin_project_lite::pin_project;
use std::cell::RefCell;
use std::rc::Rc;

use crate::subject::shareable_subject::ShareableSubject;
use crate::Event;
use crate::Observable;

pin_project! {
    /// Stream for the [`share`](RxStreamExt::share) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Shared<S: Stream> {
        inner: Rc<RefCell<ShareableSubject<S>>>,
        #[pin]
        stream: Observable<S::Item>,
    }
}

impl<S: Stream + Unpin> Shared<S> {
    pub(crate) fn new(stream: S) -> Self {
        let mut subject = ShareableSubject::new(stream);
        let stream = subject.subscribe();

        Self {
            inner: Rc::new(RefCell::new(subject)),
            stream,
        }
    }
}

impl<S: Stream + Unpin> Clone for Shared<S> {
    fn clone(&self) -> Self {
        let stream = self.inner.borrow_mut().subscribe();

        Self {
            inner: Rc::clone(&self.inner),
            stream,
        }
    }
}

impl<S: Stream + Unpin> Stream for Shared<S> {
    type Item = Event<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.borrow_mut().poll_next(cx);
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<S: Stream + Unpin> FusedStream for Shared<S>
where
    S::Item: Clone,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}
