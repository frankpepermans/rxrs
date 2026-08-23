use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pub enum ThrottleConfig {
    Leading,
    Trailing,
    All,
}

pin_project! {
    /// Stream for the [`throttle`](RxStreamExt::throttle) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Throttle<S: Stream, Fut, F> {
        config: ThrottleConfig,
        #[pin]
        stream: Fuse<S>,
        f: F,
        #[pin]
        current_interval: Option<Fut>,
        trailing: Option<S::Item>,
    }
}

impl<S: Stream, Fut, F> Throttle<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F, config: ThrottleConfig) -> Self {
        Self {
            config,
            stream: stream.fuse(),
            f,
            current_interval: None,
            trailing: None,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for Throttle<S, Fut, F>
where
    F: for<'a> FnMut(&'a S::Item) -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Throttle<S, Fut, F>
where
    F: for<'a> FnMut(&'a S::Item) -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let is_in_interval = this
            .current_interval
            .as_mut()
            .as_pin_mut()
            .map(|it| it.poll(cx).is_pending())
            .unwrap_or(false);

        if !is_in_interval && this.current_interval.is_some() {
            this.current_interval.set(None);

            if matches!(this.config, ThrottleConfig::All | ThrottleConfig::Trailing) {
                if let Some(trailing) = this.trailing.take() {
                    return Poll::Ready(Some(trailing));
                }
            }
        }

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if is_in_interval {
                    this.trailing.replace(item);
                } else {
                    this.current_interval.set(Some((this.f)(&item)));

                    if matches!(this.config, ThrottleConfig::All | ThrottleConfig::Leading) {
                        return Poll::Ready(Some(item));
                    }
                }

                cx.waker().wake_by_ref();

                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
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
    use std::future::Future;

    use futures::{executor::block_on, stream, Stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::RxExt;

    /// The source emits every [`EVENT_INTERVAL`], the throttle window spans
    /// [`THROTTLE_WINDOW`]. The two are deliberately kept away from a whole
    /// multiple of each other, so that scheduler drift cannot change how many
    /// events fall inside a single window.
    const EVENT_INTERVAL: u64 = 150;
    const THROTTLE_WINDOW: u64 = 400;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle(|_| throttle_window())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 3, 6, 9]);
        });

        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle_trailing(|_| throttle_window())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [2, 5, 8]);
        });

        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle_all(|_| throttle_window())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 2, 3, 5, 6, 8, 9]);
        });
    }

    fn throttle_window() -> impl Future<Output = futures_time::time::Instant> {
        Duration::from_millis(THROTTLE_WINDOW).into_future()
    }

    fn create_stream() -> impl Stream<Item = usize> {
        stream::unfold(0, move |count| async move {
            if count < 10 {
                Duration::from_millis(EVENT_INTERVAL).into_future().await;

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
