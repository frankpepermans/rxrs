use std::{
    collections::VecDeque,
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
    /// Stream for the [`buffer`](RxStreamExt::buffer) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Buffer<S: Stream, Fut, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        #[pin]
        current_interval: Option<Fut>,
        buffer: Option<VecDeque<S::Item>>,
    }
}

impl<S: Stream, Fut, F> Buffer<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            current_interval: None,
            buffer: None,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for Buffer<S, Fut, F>
where
    F: for<'a> FnMut(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated() && self.buffer.is_none()
    }
}

impl<S: Stream, Fut, F> Stream for Buffer<S, Fut, F>
where
    F: for<'a> FnMut(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    type Item = VecDeque<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
                match select(interval, this.stream.next()).poll_unpin(cx) {
                    Poll::Ready(it) => match it {
                        Either::Left((it, _)) => {
                            this.current_interval.set(None);

                            if it {
                                return Poll::Ready(this.buffer.take());
                            }
                        }
                        Either::Right((it, mut interval)) => match it {
                            Some(item) => {
                                interval.set((this.f)(
                                    &item,
                                    this.buffer.as_ref().map(|it| it.len()).unwrap_or_default() + 1,
                                ));

                                if let Some(it) = this.buffer.as_mut() {
                                    it.push_back(item);
                                } else {
                                    this.buffer.replace(VecDeque::from_iter([item]));
                                }
                            }
                            None => return Poll::Ready(this.buffer.take()),
                        },
                    },
                    Poll::Pending => return Poll::Pending,
                }
            } else {
                match this.stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(item)) => {
                        this.current_interval.set(Some((this.f)(
                            &item,
                            this.buffer.as_ref().map(|it| it.len()).unwrap_or_default() + 1,
                        )));

                        if let Some(it) = this.buffer.as_mut() {
                            it.push_back(item);
                        } else {
                            this.buffer.replace(VecDeque::from_iter([item]));
                        }
                    }
                    Poll::Ready(None) => return Poll::Ready(this.buffer.take()),
                    Poll::Pending => return Poll::Pending,
                }
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
    use std::collections::VecDeque;

    use futures::{executor::block_on, stream, stream::FusedStream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let all_events = stream::iter(0..=8)
                .buffer(|_, count| async move { count == 3 })
                .collect::<Vec<_>>()
                .await;

            assert_eq!(
                all_events,
                vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]]
            );
        });
    }

    #[test]
    fn is_not_terminated_while_a_partial_buffer_is_owed() {
        block_on(async {
            let mut stream =
                Box::pin(stream::iter(0..=4).buffer(|_, count| async move { count == 3 }));

            assert_eq!(stream.next().await, Some(VecDeque::from_iter([0, 1, 2])));
            assert!(!stream.is_terminated());

            assert_eq!(stream.next().await, Some(VecDeque::from_iter([3, 4])));
            assert!(stream.is_terminated());
        });
    }
}
