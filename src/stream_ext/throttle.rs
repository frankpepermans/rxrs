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
    F: for<'a> Fn(&'a S::Item) -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Throttle<S, Fut, F>
where
    F: for<'a> Fn(&'a S::Item) -> Fut,
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
    use futures::{executor::block_on, stream, Stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle(|_| Duration::from_millis(175).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 4, 8]);
        });

        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle_trailing(|_| Duration::from_millis(175).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [3, 7]);
        });

        block_on(async {
            let stream = create_stream();
            let all_events = stream
                .throttle_all(|_| Duration::from_millis(175).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 3, 4, 7, 8]);
        });
    }

    fn create_stream() -> impl Stream<Item = usize> {
        stream::unfold(0, move |count| async move {
            if count < 10 {
                Duration::from_millis(50).into_future().await;

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
