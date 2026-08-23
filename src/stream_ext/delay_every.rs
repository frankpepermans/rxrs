use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`delay`](RxStreamExt::delay) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct DelayEvery<S: Stream, Fut, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        delayed_events: VecDeque<S::Item>,
        delayed_event: Option<S::Item>,
        #[pin]
        current_interval: Option<Fut>,
        max_buffer_size: Option<usize>,
    }
}

impl<S: Stream, Fut, F> DelayEvery<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F, max_buffer_size: Option<usize>) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            delayed_events: if let Some(max_buffer_size) = &max_buffer_size {
                VecDeque::with_capacity(*max_buffer_size)
            } else {
                VecDeque::new()
            },
            delayed_event: None,
            current_interval: None,
            max_buffer_size,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for DelayEvery<S, Fut, F>
where
    F: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
            && self.delayed_events.is_empty()
            && self.delayed_event.is_none()
    }
}

impl<S: Stream, Fut, F> Stream for DelayEvery<S, Fut, F>
where
    F: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let is_done = match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                if let Some(max_buffer_size) = this.max_buffer_size {
                    while this.delayed_events.len() >= *max_buffer_size {
                        this.delayed_events.pop_front();
                    }
                }

                this.delayed_events.push_back(event);

                false
            }
            Poll::Ready(None) => true,
            Poll::Pending => false,
        };

        if this.current_interval.is_none() {
            if let Some(event) = this.delayed_events.pop_front() {
                this.current_interval.set(Some((this.f)(&event)));
                *this.delayed_event = Some(event);
            }
        }

        match this.current_interval.as_mut().as_pin_mut() {
            Some(interval) => match interval.poll(cx) {
                Poll::Ready(_) => {
                    this.current_interval.set(None);

                    Poll::Ready(this.delayed_event.take())
                }
                Poll::Pending => Poll::Pending,
            },
            None if is_done => Poll::Ready(None),
            None => Poll::Pending,
        }
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
                .delay_every(|_| Duration::from_millis(50).into_future(), None)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 2, 3]);
            assert!(now.elapsed().as_millis() >= 50 * 4);
        });
    }

    #[test]
    fn awaits_a_source_that_is_not_immediately_ready() {
        block_on(async {
            let stream = stream::unfold(0, |count| async move {
                if count < 3 {
                    Duration::from_millis(20).into_future().await;

                    Some((count, count + 1))
                } else {
                    None
                }
            });
            let all_events = stream
                .delay_every(|_| Duration::from_millis(10).into_future(), None)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 2]);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::{test_util::stuttering, RxExt};

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let events = stream::empty::<i32>()
                .delay_every(|_| Duration::from_millis(10).into_future(), None)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn every_event_is_emitted_in_order() {
        block_on(async {
            let events = stream::iter(0..=4)
                .delay_every(|_| Duration::from_millis(5).into_future(), None)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering(0..=3)
                .delay_every(|_| Duration::from_millis(5).into_future(), None)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [0, 1, 2, 3]);
        });
    }

    #[test]
    fn a_bounded_buffer_drops_the_oldest_pending_events() {
        block_on(async {
            let events = stream::iter(0..=9)
                .delay_every(|_| Duration::from_millis(5).into_future(), Some(2))
                .collect::<Vec<_>>()
                .await;

            assert!(events.len() <= 10);
            assert!(
                events.windows(2).all(|it| it[0] < it[1]),
                "events arrived out of order: {events:?}"
            );
            assert_eq!(events.last(), Some(&9));
        });
    }
}
