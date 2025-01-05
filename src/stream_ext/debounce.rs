use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    future::{select, Either},
    stream::{Fuse, FusedStream},
    FutureExt, Stream, StreamExt,
};
use futures_time::future::FutureExt as OtherFutureExt;
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`debounce`](RxStreamExt::debounce) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Debounce<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        duration: Duration,
        #[pin]
        current_interval: Option<Pin<Box<dyn Future<Output = bool>>>>,
        candidate_event: Option<S::Item>,
    }
}

impl<S: Stream> Debounce<S> {
    pub(crate) fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream: stream.fuse(),
            duration,
            current_interval: None,
            candidate_event: None,
        }
    }
}

impl<S> FusedStream for Debounce<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream> Stream for Debounce<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let duration = this.duration.as_millis().try_into().unwrap();

        fn create_interval(
            existing: bool,
            duration: u64,
        ) -> Option<Pin<Box<dyn Future<Output = bool>>>> {
            if existing {
                let deadline = futures_time::time::Duration::from_millis(duration);

                Some(Box::pin(async { true }.delay(deadline)))
            } else {
                None
            }
        }

        if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
            match select(interval, this.stream.next()).poll_unpin(cx) {
                Poll::Ready(it) => match it {
                    Either::Left(_) => {
                        *this.current_interval = create_interval(false, duration);

                        Poll::Ready(this.candidate_event.take())
                    }
                    Either::Right((it, mut interval)) => match it {
                        Some(item) => {
                            interval.set(create_interval(true, duration).unwrap());
                            *this.candidate_event = Some(item);

                            cx.waker().wake_by_ref();

                            Poll::Pending
                        }
                        None => Poll::Ready(this.candidate_event.take()),
                    },
                },
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    *this.current_interval = create_interval(true, duration);
                    *this.candidate_event = Some(item);

                    cx.waker().wake_by_ref();

                    Poll::Pending
                }
                Poll::Ready(None) => Poll::Ready(this.candidate_event.take()),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, Stream, StreamExt};
    use futures_time::{future::FutureExt, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = create_stream();

        block_on(async {
            let all_events = stream
                .debounce(std::time::Duration::from_millis(100))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [2, 6, 9]);
        });
    }

    fn create_stream() -> impl Stream<Item = usize> {
        stream::unfold(0, move |count| async move {
            if count < 10 {
                let interval = match count {
                    3 | 7 => Duration::from_millis(150),
                    _ => Duration::from_millis(50),
                };

                async { true }.delay(interval).await;

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
