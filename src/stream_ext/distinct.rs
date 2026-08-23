use std::{
    collections::HashSet,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`distinct`](RxStreamExt::distinct) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Distinct<S: Stream>
     {
        #[pin]
        stream: Fuse<S>,
        seen: HashSet<S::Item>,
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
    S::Item: Eq + Hash + Clone,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for Distinct<S>
where
    S: Stream,
    S::Item: Eq + Hash + Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(event)) => {
                    if this.seen.insert(event.clone()) {
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
            let stream = stream::iter([1, 1, 2, 1, 3, 2, 4, 5]);
            let all_events = stream.distinct().collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 2, 3, 4, 5]);
        });
    }

    #[test]
    fn compares_values() {
        block_on(async {
            let stream = stream::iter(["a", "b", "a", "c", "b"]);
            let all_events = stream.distinct().collect::<Vec<_>>().await;

            assert_eq!(all_events, ["a", "b", "c"]);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{test_util::stuttering, RxExt};

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let events = stream::empty::<i32>().distinct().collect::<Vec<_>>().await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn a_single_event_is_emitted() {
        block_on(async {
            let events = stream::iter([1]).distinct().collect::<Vec<_>>().await;

            assert_eq!(events, [1]);
        });
    }

    #[test]
    fn an_all_duplicate_source_emits_once() {
        block_on(async {
            let events = stream::iter([7; 32]).distinct().collect::<Vec<_>>().await;

            assert_eq!(events, [7]);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering([1, 1, 2, 3, 2])
                .distinct()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2, 3]);
        });
    }

    #[test]
    fn treats_unequal_values_as_distinct() {
        block_on(async {
            let events = stream::iter([(1, "a"), (1, "b"), (1, "a")])
                .distinct()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, "a"), (1, "b")]);
        });
    }
}
