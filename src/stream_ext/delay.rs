use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    FutureExt, Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`delay`](RxStreamExt::delay) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Delay<S: Stream, Fut, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        #[pin]
        interval: Option<Fut>,
        did_delay: bool,
    }
}

impl<S: Stream, Fut, F> Delay<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            interval: None,
            did_delay: false,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for Delay<S, Fut, F>
where
    F: FnMut() -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Delay<S, Fut, F>
where
    F: FnMut() -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if !*this.did_delay {
            *this.did_delay = true;

            this.interval.set(Some((this.f)()));
        }

        if let Some(mut interval) = this.interval.as_mut().as_pin_mut() {
            match interval.poll_unpin(cx) {
                Poll::Ready(_) => this.interval.set(None),
                Poll::Pending => return Poll::Pending,
            }
        }

        this.stream.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.stream.size_hint();
        // we know for sure that the final event (if any) will always emit,
        // any other events depend on a time interval and must be discarded.
        let lower = if lower > 0 { 1 } else { 0 };

        (lower, upper)
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use futures::{executor::block_on, stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let now = Instant::now();
            let all_events = stream::iter(0..=3)
                .delay(|| Duration::from_millis(100).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 2, 3]);
            assert!(now.elapsed().as_millis() >= 100);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use std::time::Instant;

    use futures::{executor::block_on, stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::{test_util::stuttering, RxExt};

    #[test]
    fn an_empty_source_still_waits_out_the_delay() {
        block_on(async {
            let now = Instant::now();
            let events = stream::empty::<i32>()
                .delay(|| Duration::from_millis(50).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
            assert!(now.elapsed().as_millis() >= 50);
        });
    }

    #[test]
    fn the_first_event_is_delayed_too() {
        block_on(async {
            let now = Instant::now();
            let mut stream =
                Box::pin(stream::iter(0..=3).delay(|| Duration::from_millis(50).into_future()));

            assert_eq!(stream.next().await, Some(0));
            assert!(now.elapsed().as_millis() >= 50);
        });
    }

    #[test]
    fn the_delay_applies_once_and_not_per_event() {
        block_on(async {
            let now = Instant::now();
            let events = stream::iter(0..=3)
                .delay(|| Duration::from_millis(50).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [0, 1, 2, 3]);
            assert!(now.elapsed().as_millis() < 200);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering([1, 2])
                .delay(|| Duration::from_millis(10).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2]);
        });
    }
}
