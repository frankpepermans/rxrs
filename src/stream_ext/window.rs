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
    F: for<'a> FnMut(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated() && self.buffer.is_none()
    }
}

impl<S: Stream, Fut, F> Stream for Window<S, Fut, F>
where
    F: for<'a> FnMut(&'a S::Item, usize) -> Fut,
    Fut: Future<Output = bool>,
{
    type Item = Iter<IntoIter<S::Item>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if let Some(interval) = this.current_interval.as_mut().as_pin_mut() {
                match select(interval, this.stream.next()).poll_unpin(cx) {
                    Poll::Ready(it) => match it {
                        Either::Left((it, _)) => {
                            this.current_interval.set(None);

                            if it {
                                return Poll::Ready(this.buffer.take().map(stream::iter));
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
                            }
                            None => return Poll::Ready(this.buffer.take().map(stream::iter)),
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
                            it.push(item);
                        } else {
                            this.buffer.replace(vec![item]);
                        }
                    }
                    Poll::Ready(None) => return Poll::Ready(this.buffer.take().map(stream::iter)),
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

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{test_util::stuttering, RxExt};

    async fn flatten(
        stream: impl futures::Stream<Item = futures::stream::Iter<std::vec::IntoIter<i32>>>,
    ) -> Vec<Vec<i32>> {
        stream
            .then(|it| async { it.collect::<Vec<_>>().await })
            .collect::<Vec<_>>()
            .await
    }

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let events =
                flatten(stream::empty::<i32>().window(|_, count| async move { count == 2 })).await;

            assert!(events.is_empty());
        });
    }

    #[test]
    fn a_trailing_partial_window_is_still_emitted() {
        block_on(async {
            let events =
                flatten(stream::iter(0..=3).window(|_, count| async move { count == 3 })).await;

            assert_eq!(events, [vec![0, 1, 2], vec![3]]);
        });
    }

    #[test]
    fn a_predicate_that_never_closes_yields_one_window() {
        block_on(async {
            let events = flatten(stream::iter(0..=3).window(|_, _| async { false })).await;

            assert_eq!(events, [vec![0, 1, 2, 3]]);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events =
                flatten(stuttering(0..=3).window(|_, count| async move { count == 2 })).await;

            assert_eq!(events, [vec![0, 1], vec![2, 3]]);
        });
    }
}
