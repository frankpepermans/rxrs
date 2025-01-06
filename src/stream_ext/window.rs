use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    vec::IntoIter,
};

use futures::{
    future::{select, Either},
    stream::{self, Fuse, FusedStream, Iter},
    FutureExt, Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`window`](RxStreamExt::window) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Window<S: Stream, Fut, F> {
        #[pin]
        stream: Fuse<S>,
        f: F,
        #[pin]
        current_interval: Option<Fut>,
        buffer: Option<Vec<S::Item>>,
    }
}

impl<S: Stream, Fut, F> Window<S, Fut, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            f,
            current_interval: None,
            buffer: None,
        }
    }
}

impl<S: Stream, Fut, F> FusedStream for Window<S, Fut, F>
where
    F: for<'a> Fn(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream, Fut, F> Stream for Window<S, Fut, F>
where
    F: for<'a> Fn(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    type Item = Iter<IntoIter<S::Item>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
            match select(interval, this.stream.next()).poll_unpin(cx) {
                Poll::Ready(it) => match it {
                    Either::Left((it, _)) => {
                        this.current_interval.set(None);

                        if it {
                            Poll::Ready(this.buffer.take().map(stream::iter))
                        } else {
                            cx.waker().wake_by_ref();

                            Poll::Pending
                        }
                    }
                    Either::Right((it, mut interval)) => match it {
                        Some(item) => {
                            interval.set((this.f)(
                                &item,
                                this.buffer.as_ref().map(|it| it.len()).unwrap_or_default() + 1,
                            ));

                            if let Some(it) = this.buffer.as_mut() {
                                it.push(item);
                            } else {
                                this.buffer.replace(vec![item]);
                            }

                            cx.waker().wake_by_ref();

                            Poll::Pending
                        }
                        None => Poll::Ready(this.buffer.take().map(stream::iter)),
                    },
                },
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    this.current_interval.set(Some((this.f)(
                        &item,
                        this.buffer.as_ref().map(|it| it.len()).unwrap_or_default() + 1,
                    )));

                    if let Some(it) = this.buffer.as_mut() {
                        it.push(item);
                    } else {
                        this.buffer.replace(vec![item]);
                    }

                    cx.waker().wake_by_ref();

                    Poll::Pending
                }
                Poll::Ready(None) => Poll::Ready(this.buffer.take().map(stream::iter)),
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
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let all_events = stream::iter(0..=8)
                .window(|_, count| async move { count == 3 })
                .enumerate()
                .flat_map(|(index, it)| it.map(move |it| (index, it)))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(
                all_events,
                vec![
                    (0, 0),
                    (0, 1),
                    (0, 2),
                    (1, 3),
                    (1, 4),
                    (1, 5),
                    (2, 6),
                    (2, 7),
                    (2, 8)
                ]
            );
        });
    }
}
