use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    future::{select, Either},
    stream::{Fuse, FusedStream},
    FutureExt, Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`debounce`](RxStreamExt::debounce) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Debounce<S: Stream, Fut, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        #[pin]
        current_interval: Option<Fut>,
        candidate_event: Option<S::Item>,
    }
}

impl<S: Stream, Fut, F> Debounce<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            current_interval: None,
            candidate_event: None,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for Debounce<S, Fut, F>
where
    F: for<'a> Fn(&'a S::Item) -> Fut,
    Fut: Future,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Debounce<S, Fut, F>
where
    F: for<'a> Fn(&'a S::Item) -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
            match select(interval, this.stream.next()).poll_unpin(cx) {
                Poll::Ready(it) => match it {
                    Either::Left(_) => {
                        this.current_interval.set(None);

                        Poll::Ready(this.candidate_event.take())
                    }
                    Either::Right((it, mut interval)) => match it {
                        Some(item) => {
                            interval.set((this.f)(&item));
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
                    this.current_interval.set(Some((this.f)(&item)));
                    *this.candidate_event = Some(item);

                    cx.waker().wake_by_ref();

                    Poll::Pending
                }
                Poll::Ready(None) => Poll::Ready(this.candidate_event.take()),
                Poll::Pending => Poll::Pending,
            }
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
    use futures_time::{future::FutureExt, time::Duration};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = create_stream();

        block_on(async {
            let all_events = stream
                .debounce(|_| {
                    async {}.delay(futures_time::time::Duration::from_millis(
                        std::time::Duration::from_millis(100)
                            .as_millis()
                            .try_into()
                            .unwrap(),
                    ))
                })
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
