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
    }
}

impl<S: Stream, F> InspectDone<S, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
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
            (this.f)();
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
