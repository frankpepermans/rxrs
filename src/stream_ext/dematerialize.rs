use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::Fuse, stream::FusedStream, Stream, StreamExt};
use pin_project_lite::pin_project;

use crate::Notification;

pin_project! {
    /// Stream for the [`dematerialize`](RxStreamExt::dematerialize) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Dematerialize<S: Stream<Item = Notification<T>>, T> {
        #[pin]
        stream: Fuse<S>,
        item: PhantomData<T>,
    }
}

impl<S: Stream<Item = Notification<T>>, T> Dematerialize<S, T> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            item: PhantomData,
        }
    }
}

impl<S: FusedStream<Item = Notification<T>>, T> FusedStream for Dematerialize<S, T> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream<Item = Notification<T>>, T> Stream for Dematerialize<S, T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => match event {
                Notification::Next(event) => Poll::Ready(Some(event)),
                Notification::Complete => Poll::Ready(None),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();

        (a.saturating_sub(1), b.map(|it| it.saturating_sub(1)))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, Stream, StreamExt};

    use crate::{Notification, RxExt};

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(1..=2);
            let all_events = stream
                .materialize()
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [1, 2]);
        });
    }

    #[test]
    fn size_hint_does_not_underflow_on_an_empty_source() {
        let stream = stream::empty::<Notification<i32>>().dematerialize();

        assert_eq!(stream.size_hint(), (0, Some(0)));
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{test_util::stuttering, Notification, RxExt};

    #[test]
    fn an_empty_source_emits_nothing() {
        block_on(async {
            let events = stream::empty::<Notification<i32>>()
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn a_completion_event_ends_the_stream_early() {
        block_on(async {
            let events = stream::iter([
                Notification::Next(1),
                Notification::Complete,
                Notification::Next(2),
            ])
            .dematerialize()
            .collect::<Vec<_>>()
            .await;

            assert_eq!(events, [1]);
        });
    }

    #[test]
    fn a_source_without_a_completion_event_still_ends() {
        block_on(async {
            let events = stream::iter([Notification::Next(1), Notification::Next(2)])
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2]);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering([Notification::Next(1), Notification::Complete])
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1]);
        });
    }

    #[test]
    fn round_trips_with_materialize() {
        block_on(async {
            let events = stream::iter(0..=3)
                .materialize()
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [0, 1, 2, 3]);
        });
    }
}
