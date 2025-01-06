use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::Fuse, stream::FusedStream, Stream, StreamExt};
use pin_project_lite::pin_project;

use crate::Notification;

pin_project! {
    /// Stream for the [`start_with`](RxStreamExt::start_with) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Dematerialize<S: Stream<Item = Notification<T>>, T> {
        #[pin]
        stream: Fuse<S>,
    }
}

impl<S: Stream<Item = Notification<T>>, T> Dematerialize<S, T> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
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

        (a - 1, b.map(|it| it - 1))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{Notification, RxExt};

    #[test]
    fn smoke() {
        let stream = stream::iter(1..=2);

        block_on(async {
            let all_events = stream.materialize().collect::<Vec<_>>().await;

            assert_eq!(
                all_events,
                [
                    Notification::Next(1),
                    Notification::Next(2),
                    Notification::Complete
                ]
            );
        });
    }
}
