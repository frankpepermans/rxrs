use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`start_with`](RxStreamExt::start_with) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct EndWith<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        value: Option<VecDeque<S::Item>>,
    }
}

impl<S: Stream> EndWith<S> {
    pub(crate) fn new<I: IntoIterator<Item = S::Item>>(stream: S, value: I) -> Self {
        let items = VecDeque::from_iter(value);

        Self {
            stream: stream.fuse(),
            value: Some(items),
        }
    }
}

impl<S: FusedStream> FusedStream for EndWith<S> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S: Stream> Stream for EndWith<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(None) => {
                if let Some(value) = this.value.as_mut() {
                    if let Some(event) = value.pop_front() {
                        Poll::Ready(Some(event))
                    } else {
                        *this.value = None;
                        Poll::Ready(None)
                    }
                } else {
                    Poll::Ready(None)
                }
            }
            it => it,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.value.as_ref().map(|it| it.len()).unwrap_or_default();
        let (a, b) = self.stream.size_hint();

        (a + len, b.map(|it| it + len))
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(1..=5);
            let all_events = stream.end_with([0]).collect::<Vec<_>>().await;

            assert_eq!(all_events, [1, 2, 3, 4, 5, 0]);
        });
    }
}
