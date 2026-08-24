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
    /// Stream for the [`with_latest_from`](RxStreamExt::with_latest_from) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct WithLatestFrom<S1: Stream, S2: Stream> {
        #[pin]
        stream: Fuse<S1>,
        #[pin]
        other: Fuse<S2>,
        latest: Option<S2::Item>,
    }
}

impl<S1: Stream, S2: Stream> WithLatestFrom<S1, S2> {
    pub(crate) fn new(stream: S1, other: S2) -> Self {
        Self {
            stream: stream.fuse(),
            other: other.fuse(),
            latest: None,
        }
    }
}

impl<S1: Stream, S2: Stream> FusedStream for WithLatestFrom<S1, S2>
where
    S2::Item: Clone,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S1: Stream, S2: Stream> Stream for WithLatestFrom<S1, S2>
where
    S2::Item: Clone,
{
    type Item = (S1::Item, S2::Item);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if let Poll::Ready(Some(event)) = this.other.as_mut().poll_next(cx) {
                *this.latest = Some(event);
            }

            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(event)) => {
                    if let Some(latest) = this.latest.clone() {
                        return Poll::Ready(Some((event, latest)));
                    }
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.stream.size_hint();

        (0, upper)
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{
        test_util::{never, stuttering},
        RxExt,
    };

    #[test]
    fn smoke() {
        block_on(async {
            let events = stream::iter(0..=3)
                .with_latest_from(stream::iter([9]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(0, 9), (1, 9), (2, 9), (3, 9)]);
        });
    }

    #[test]
    fn only_the_source_drives_emissions() {
        block_on(async {
            let events = stream::iter([1, 2])
                .with_latest_from(stream::iter([7, 8, 9]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events.len(), 2);
            assert_eq!(events.iter().map(|it| it.0).collect::<Vec<_>>(), [1, 2]);
        });
    }

    #[test]
    fn source_events_before_the_first_other_event_are_dropped() {
        block_on(async {
            let events = stream::iter([1, 2, 3])
                .with_latest_from(stuttering([9]))
                .collect::<Vec<_>>()
                .await;

            assert!(events.len() < 3, "expected early events to be dropped");
            assert!(events.iter().all(|it| it.1 == 9));
        });
    }

    #[test]
    fn an_other_that_never_emits_yields_nothing() {
        block_on(async {
            let events = stream::iter([1, 2, 3])
                .with_latest_from(never::<i32>())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn an_exhausted_other_keeps_supplying_its_last_event() {
        block_on(async {
            let events = stuttering([1, 2, 3])
                .with_latest_from(stream::iter([9]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 9), (2, 9), (3, 9)]);
        });
    }

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let events = stream::empty::<i32>()
                .with_latest_from(stream::iter([9]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn the_output_ends_with_the_source_even_if_the_other_never_does() {
        block_on(async {
            let events = stuttering([1, 2])
                .with_latest_from(stream::iter([9]).chain(never()))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 9), (2, 9)]);
        });
    }
}
