use std::{
    collections::HashSet,
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
    pub struct Distinct<S: Stream>
     {
        #[pin]
        stream: Fuse<S>,
        #[pin]
        seen: HashSet<u64>,
    }
}

impl<S: Stream> Distinct<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            seen: HashSet::new(),
        }
    }
}

impl<S> FusedStream for Distinct<S>
where
    S: FusedStream,
    S::Item: Hash,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for Distinct<S>
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

                let should_emit = this.seen.as_mut().get_mut().insert(hasher.finish());

                if should_emit {
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
        let stream = stream::iter([1, 1, 2, 1, 3, 2, 4, 5]);

        block_on(async {
            let all_events = stream.distinct().collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 2, 3, 4, 5]);
        });
    }
}
