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
    /// Stream for the [`distinct_until_changed`](RxStreamExt::distinct_until_changed) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct DistinctUntilChanged<S: Stream>
     {
        #[pin]
        stream: Fuse<S>,
        previous: Option<S::Item>,
    }
}

impl<S: Stream> DistinctUntilChanged<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            previous: None,
        }
    }
}

impl<S> FusedStream for DistinctUntilChanged<S>
where
    S: FusedStream,
    S::Item: PartialEq + Clone,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for DistinctUntilChanged<S>
where
    S: Stream,
    S::Item: PartialEq + Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(event)) => {
                    if this.previous.as_ref() != Some(&event) {
                        *this.previous = Some(event.clone());

                        return Poll::Ready(Some(event));
                    }
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.stream.size_hint();
        let lower = if lower > 0 { 1 } else { 0 };

        (lower, upper)
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter([1, 1, 2, 3, 3, 3, 4, 5]);
            let all_events = stream.distinct_until_changed().collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 2, 3, 4, 5]);
        });
    }

    #[test]
    fn accepts_events_that_are_not_hashable() {
        block_on(async {
            let stream = stream::iter([1.0f64, 1.0, 2.5, 2.5, 1.0]);
            let all_events = stream.distinct_until_changed().collect::<Vec<_>>().await;

            assert_eq!(all_events, [1.0, 2.5, 1.0]);
        });
    }
}
