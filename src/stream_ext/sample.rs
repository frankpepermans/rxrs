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
        self.stream.is_terminated() || self.sampler.is_terminated()
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
            Poll::Ready(None) => Poll::Ready(this.latest_event.take()),
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

    #[test]
    fn smoke() {
        let stream = futures_time::stream::interval(Duration::from_millis(20))
            .take(6)
            .enumerate()
            .map(|(index, _)| index);
        let sampler = futures_time::stream::interval(Duration::from_millis(50)).take(6);

        block_on(async {
            let all_events = stream.sample(sampler).collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 3, 5]);
        });
    }
}
