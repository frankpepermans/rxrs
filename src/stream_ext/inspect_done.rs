use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`inspect_done`](RxStreamExt::inspect_done) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct InspectDone<S: Stream, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        did_complete: bool,
    }
}

impl<S: Stream, F> InspectDone<S, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            did_complete: false,
        }
    }
}

impl<S: Stream, F> FusedStream for InspectDone<S, F>
where
    F: FnMut(),
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, F> Stream for InspectDone<S, F>
where
    F: FnMut(),
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let poll_next = this.stream.poll_next(cx);

        if let Poll::Ready(None) = &poll_next {
            if !*this.did_complete {
                *this.did_complete = true;

                (this.f)();
            }
        }

        poll_next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let mut is_done = false;
            let all_events = stream::iter(0..=3)
                .inspect_done(|| is_done = true)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 2, 3]);
            assert!(is_done);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use std::cell::Cell;

    use futures::{executor::block_on, stream, StreamExt};

    use crate::{test_util::stuttering, RxExt};

    #[test]
    fn fires_once_for_an_empty_source() {
        block_on(async {
            let calls = Cell::new(0);
            let events = stream::empty::<i32>()
                .inspect_done(|| calls.set(calls.get() + 1))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
            assert_eq!(calls.get(), 1);
        });
    }

    #[test]
    fn fires_exactly_once_after_the_final_event() {
        block_on(async {
            let calls = Cell::new(0);
            let mut stream =
                Box::pin(stream::iter([1, 2]).inspect_done(|| calls.set(calls.get() + 1)));

            assert_eq!(stream.next().await, Some(1));
            assert_eq!(calls.get(), 0);

            assert_eq!(stream.next().await, Some(2));
            assert_eq!(calls.get(), 0);

            assert_eq!(stream.next().await, None);
            assert_eq!(calls.get(), 1);

            assert_eq!(stream.next().await, None);
            assert_eq!(calls.get(), 1);
        });
    }

    #[test]
    fn does_not_fire_while_a_source_is_merely_pending() {
        block_on(async {
            let calls = Cell::new(0);
            let events = stuttering([1, 2])
                .inspect_done(|| calls.set(calls.get() + 1))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2]);
            assert_eq!(calls.get(), 1);
        });
    }
}
