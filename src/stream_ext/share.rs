use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures::stream::Fuse;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;
use pin_project_lite::pin_project;
use std::cell::RefCell;
use std::rc::Rc;

use crate::subject::shareable_subject::ShareableSubject;
use crate::subject::Subject;
use crate::Event;
use crate::Observable;

pin_project! {
    /// Stream for the [`share`](RxStreamExt::share) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Shared<S: Stream, Sub: Subject<Item = S::Item>> {
        inner: Rc<RefCell<ShareableSubject<S, Sub>>>,
        #[pin]
        stream: Fuse<Observable<S::Item>>,
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Shared<S, Sub> {
    pub(crate) fn new(stream: S, subject: Sub) -> Self {
        let mut subject = ShareableSubject::new(stream, subject);
        let stream = subject.subscribe().fuse();

        Self {
            inner: Rc::new(RefCell::new(subject)),
            stream,
        }
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Clone for Shared<S, Sub> {
    fn clone(&self) -> Self {
        let stream = self.inner.borrow_mut().subscribe().fuse();

        Self {
            inner: Rc::clone(&self.inner),
            stream,
        }
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Stream for Shared<S, Sub> {
    type Item = Event<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.borrow_mut().poll_next(cx);
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> FusedStream for Shared<S, Sub> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, future::join, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = stream::iter(1usize..=3usize);
        let s1 = stream.share();
        let s2 = s1.clone();

        block_on(async {
            let (a, b) = join(s1.collect::<Vec<_>>(), s2.collect::<Vec<_>>()).await;

            assert_eq!(a, [1.into(), 2.into(), 3.into()]);
            assert_eq!(b, [1.into(), 2.into(), 3.into()]);
        });
    }
}
