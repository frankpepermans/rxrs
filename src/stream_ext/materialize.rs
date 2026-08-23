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

    use crate::{Notification, RxExt};

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(1..=2);
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

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, Stream, StreamExt};

    use crate::{test_util::stuttering, Notification, RxExt};

    #[test]
    fn an_empty_source_still_reports_completion() {
        block_on(async {
            let events = stream::empty::<i32>()
                .materialize()
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [Notification::Complete]);
        });
    }

    #[test]
    fn completion_is_only_reported_once() {
        block_on(async {
            let mut stream = Box::pin(stream::iter([1]).materialize());

            assert_eq!(stream.next().await, Some(Notification::Next(1)));
            assert_eq!(stream.next().await, Some(Notification::Complete));
            assert_eq!(stream.next().await, None);
            assert_eq!(stream.next().await, None);
        });
    }

    #[test]
    fn survives_a_source_that_is_not_always_ready() {
        block_on(async {
            let events = stuttering([1, 2]).materialize().collect::<Vec<_>>().await;

            assert_eq!(
                events,
                [
                    Notification::Next(1),
                    Notification::Next(2),
                    Notification::Complete
                ]
            );
        });
    }

    #[test]
    fn size_hint_accounts_for_the_completion_event() {
        let stream = stream::iter([1, 2, 3]).materialize();

        assert_eq!(stream.size_hint(), (4, Some(4)));
    }
}
