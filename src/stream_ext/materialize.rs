use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::Notification;

pin_project! {
    /// Stream for the [`start_with`](RxStreamExt::start_with) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Materialize<S: Stream> {
        #[pin]
        stream: S,
        did_complete: bool,
    }
}

impl<S: Stream> Materialize<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream,
            did_complete: false,
        }
    }
}

impl<S: FusedStream> FusedStream for Materialize<S> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream> Stream for Materialize<S> {
    type Item = Notification<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => Poll::Ready(Some(Notification::Next(event))),
            Poll::Ready(None) => {
                if *this.did_complete {
                    Poll::Ready(None)
                } else {
                    *this.did_complete = true;
                    Poll::Ready(Some(Notification::Complete))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();

        (a + 1, b.map(|it| it + 1))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = stream::iter(1..=2);

        block_on(async {
            let all_events = stream
                .materialize()
                .dematerialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [1, 2]);
        });
    }
}
