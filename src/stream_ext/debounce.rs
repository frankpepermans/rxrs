use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    future::{select, Either, FusedFuture},
    stream::{Fuse, FusedStream},
    FutureExt, Stream, StreamExt,
};
use futures_timer::Delay;
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`debounce`](RxStreamExt::debounce) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Debounce<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        duration: Duration,
        #[pin]
        current_interval: Option<Delay>,
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

        if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
            let mut switch = select(interval, this.stream.next());

            match switch.poll_unpin(cx) {
                Poll::Ready(it) => match it {
                    Either::Left((_, other)) => {
                        if other.is_terminated() {
                            this.current_interval.set(None);
                            return Poll::Ready(this.candidate_event.take());
                        }
                    }
                    Either::Right((it, other)) => match it {
                        Some(item) => match other.poll(cx) {
                            Poll::Ready(_) => {
                                this.current_interval.set(None);
                                return Poll::Ready(this.candidate_event.take());
                            }
                            Poll::Pending => {
                                this.current_interval.set(Some(Delay::new(*this.duration)));
                                *this.candidate_event = Some(item);
                            }
                        },
                        None => return Poll::Ready(this.candidate_event.take()),
                    },
                },
                Poll::Pending => return Poll::Pending,
            }
        } else {
            match this.stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    this.current_interval.set(Some(Delay::new(*this.duration)));
                    *this.candidate_event = Some(item);
                }
                Poll::Ready(None) => return Poll::Ready(this.candidate_event.take()),
                Poll::Pending => return Poll::Pending,
            };
        };

        cx.waker().wake_by_ref();

        Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use std::{thread, time::Duration};

    use futures::{executor::block_on, stream, Stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = create_stream();

        block_on(async {
            let all_events = stream
                .debounce(Duration::from_millis(100))
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

                thread::sleep(interval);

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
