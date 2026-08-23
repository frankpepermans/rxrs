use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

use crate::EventLite;

pin_project! {
    /// Stream for the [`pairwise`](RxStreamExt::pairwise) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Pairwise<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        previous: Option<Arc<S::Item>>,
    }
}

impl<S: Stream> Pairwise<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            previous: None,
        }
    }
}

impl<S> FusedStream for Pairwise<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for Pairwise<S>
where
    S: Stream,
{
    type Item = (EventLite<S::Item>, EventLite<S::Item>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(event)) => {
                    let next = Arc::new(event);

                    if let Some(prev) = this.previous.replace(Arc::clone(&next)) {
                        return Poll::Ready(Some((EventLite(prev), EventLite(next))));
                    }
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();
        let lower = if a > 0 { a - 1 } else { 0 };

        (lower, b.map(|it| if it > 0 { it - 1 } else { 0 }))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(0..=5);
            let all_events = stream
                .pairwise()
                .map(|(prev, next)| (*prev, *next))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
        });
    }

    /// Events stay valid when the consumer holds on to them, instead of
    /// dropping them before the next poll.
    #[test]
    fn retained_events() {
        block_on(async {
            let all_events = stream::iter(0..=5).pairwise().collect::<Vec<_>>().await;

            assert_eq!(all_events.len(), 5);
            assert_eq!(*all_events[0].0, 0);
            assert_eq!(*all_events[4].1, 5);
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
            let events = stream::empty::<i32>().pairwise().collect::<Vec<_>>().await;

            assert_eq!(events.len(), 0);
        });
    }

    #[test]
    fn a_single_event_has_nothing_to_pair_with() {
        block_on(async {
            let events = stream::iter([1]).pairwise().collect::<Vec<_>>().await;

            assert_eq!(events.len(), 0);
        });
    }

    #[test]
    fn two_events_form_one_pair() {
        block_on(async {
            let events = stream::iter([1, 2])
                .pairwise()
                .map(|(a, b)| (*a, *b))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 2)]);
        });
    }

    #[test]
    fn each_event_appears_as_both_halves_of_a_pair() {
        block_on(async {
            let events = stream::iter([1, 2, 3]).pairwise().collect::<Vec<_>>().await;

            assert!(std::ptr::eq(
                events[0].1.borrow_value(),
                events[1].0.borrow_value()
            ));
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering([1, 2, 3])
                .pairwise()
                .map(|(a, b)| (*a, *b))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 2), (2, 3)]);
        });
    }
}
