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
    F: Fn() -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Delay<S, Fut, F>
where
    F: Fn() -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.interval.as_mut().as_pin_mut() {
            Some(mut interval) => match interval.poll_unpin(cx) {
                Poll::Ready(_) => {
                    this.interval.set(None);
                }
                Poll::Pending => {
                    cx.waker().wake_by_ref();

                    return Poll::Pending;
                }
            },
            None => {
                if !*this.did_delay {
                    *this.did_delay = true;
                    this.interval.set(Some((this.f)()))
                }
            }
        };

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
    use std::time::SystemTime;

    use futures::{executor::block_on, stream, StreamExt};
    use futures_time::{future::IntoFuture, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let now = SystemTime::now();
            let all_events = stream::iter(0..=3)
                .delay(|| Duration::from_millis(100).into_future())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 2, 3]);
            assert!(now.elapsed().unwrap().as_millis() >= 100);
        });
    }
}
