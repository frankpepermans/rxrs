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
    /// Stream for the [`sample`](RxStreamExt::sample) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Sample<S1: Stream, S2: Stream> {
        #[pin]
        stream: Fuse<S1>,
        #[pin]
        sampler: Fuse<S2>,
        latest_event: Option<S1::Item>,
    }
}

impl<S1: Stream, S2: Stream> Sample<S1, S2> {
    pub(crate) fn new(stream: S1, sampler: S2) -> Self {
        Self {
            stream: stream.fuse(),
            sampler: sampler.fuse(),
            latest_event: None,
        }
    }
}

impl<S1: Stream, S2: Stream> FusedStream for Sample<S1, S2>
where
    S1: FusedStream,
    S2: FusedStream,
{
    fn is_terminated(&self) -> bool {
        (self.stream.is_terminated() || self.sampler.is_terminated()) && self.latest_event.is_none()
    }
}

impl<S1: Stream, S2: Stream> Stream for Sample<S1, S2> {
    type Item = S1::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if let Poll::Ready(Some(event)) = this.stream.poll_next(cx) {
            this.latest_event.replace(event);

            cx.waker().wake_by_ref();
        }

        match this.sampler.poll_next(cx) {
            Poll::Ready(Some(_)) => {
                if this.latest_event.is_some() {
                    Poll::Ready(this.latest_event.take())
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
        let (lower_left, upper_left) = self.stream.size_hint();
        let (lower_right, upper_right) = self.sampler.size_hint();

        (lower_left.min(lower_right), upper_left.max(upper_right))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, StreamExt};
    use futures_time::time::Duration;

    use crate::RxExt;

    /// The sampler ticks at 2⅓ times the event interval, so that a sample
    /// never coincides with an event and scheduler drift cannot decide which
    /// of two events is the latest one.
    const EVENT_INTERVAL: u64 = 120;
    const SAMPLE_INTERVAL: u64 = 280;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = futures_time::stream::interval(Duration::from_millis(EVENT_INTERVAL))
                .take(6)
                .enumerate()
                .map(|(index, _)| index);
            let sampler =
                futures_time::stream::interval(Duration::from_millis(SAMPLE_INTERVAL)).take(3);
            let all_events = stream.sample(sampler).collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 3, 5]);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};
    use futures_time::time::Duration;

    use crate::RxExt;

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let sampler = futures_time::stream::interval(Duration::from_millis(5)).take(3);
            let events = stream::empty::<i32>()
                .sample(sampler)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn a_sampler_that_never_ticks_emits_nothing() {
        block_on(async {
            let events = stream::iter(0..=3)
                .sample(stream::empty::<()>())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn only_the_most_recent_event_survives_a_tick() {
        block_on(async {
            let sampler = futures_time::stream::interval(Duration::from_millis(30)).take(1);
            let events = stream::iter(0..=9)
                .sample(sampler)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [9]);
        });
    }
}
