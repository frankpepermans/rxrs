use std::{
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

use crate::EventLite;

pin_project! {
    /// Stream for the [`pairwise`](RxStreamExt::pairwise) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Pairwise<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        previous: Option<Rc<S::Item>>,
    }
}

impl<S: Stream> Pairwise<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: stream.fuse(),
            previous: None,
        }
    }
}

impl<S> FusedStream for Pairwise<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for Pairwise<S>
where
    S: Stream,
{
    type Item = (S::Item, EventLite<S::Item>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                let next = Rc::new(event);

                if let Some(prev) = this.previous.replace(Rc::clone(&next)) {
                    if let Ok(prev) = Rc::try_unwrap(prev) {
                        Poll::Ready(Some((prev, EventLite(next))))
                    } else {
                        unreachable!()
                    }
                } else {
                    cx.waker().wake_by_ref();

                    Poll::Pending
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();
        let lower = if a > 0 { a - 1 } else { 0 };

        (lower, b.map(|it| if it > 0 { it - 1 } else { 0 }))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(0..=5);
            let all_events = stream
                .pairwise()
                .map(|(prev, next)| (prev, *next))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
        });
    }
}
