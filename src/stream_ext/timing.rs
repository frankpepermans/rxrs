use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`timing`](RxStreamExt::timing) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Timing<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        last_time: Option<Instant>,
    }
}

impl<S: Stream> Timing<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            last_time: None,
        }
    }
}

impl<S: Stream> FusedStream for Timing<S> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream> Stream for Timing<S> {
    type Item = Timed<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(event)) => {
                let timestamp = Instant::now();
                let interval = this.last_time.map(|it| timestamp.duration_since(it));

                *this.last_time = Some(timestamp);

                Poll::Ready(Some(Timed {
                    event,
                    timestamp,
                    interval,
                }))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[derive(Debug, Clone)]
pub struct Timed<T> {
    pub event: T,
    pub timestamp: Instant,
    pub interval: Option<Duration>,
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use futures::{executor::block_on, stream, Stream, StreamExt};
    use futures_time::{future::FutureExt, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = create_stream();

        block_on(async {
            let start = Instant::now();
            let all_events = stream.timing().collect::<Vec<_>>().await;
            let timestamps = all_events
                .iter()
                .map(|it| it.timestamp)
                .enumerate()
                .collect::<Vec<_>>();
            let intervals = all_events
                .iter()
                .map(|it| it.interval)
                .enumerate()
                .collect::<Vec<_>>();

            for (index, timestamp) in timestamps {
                assert!(
                    timestamp.duration_since(start).as_millis() >= (50 * index).try_into().unwrap()
                );
            }

            for (index, interval) in intervals {
                if index == 0 {
                    assert!(interval.is_none());
                } else {
                    assert!(interval.expect("interval is None!").as_millis() >= 50);
                }
            }
        });
    }

    fn create_stream() -> impl Stream<Item = usize> {
        stream::unfold(0, move |count| async move {
            if count < 10 {
                async { true }.delay(Duration::from_millis(50)).await;

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
