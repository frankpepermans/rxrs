use std::{
    hash::{DefaultHasher, Hash, Hasher},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`pairwise`](RxStreamExt::pairwise) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct DistinctUntilChanged<S: Stream>
     {
        #[pin]
        stream: Fuse<S>,
        #[pin]
        previous: Option<u64>,
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
    S::Item: Hash,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for DistinctUntilChanged<S>
where
    S: Stream,
    S::Item: Hash,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(event)) => {
                let mut hasher = DefaultHasher::new();

                event.hash(&mut hasher);

                let hash = hasher.finish();
                let should_emit = match this.previous.as_ref().get_ref() {
                    Some(it) => *it != hash,
                    None => true,
                };

                if should_emit {
                    this.previous.set(Some(hasher.finish()));

                    Poll::Ready(Some(event))
                } else {
                    cx.waker().wake_by_ref();

                    Poll::Pending
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
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
}
