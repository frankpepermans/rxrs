use std::{
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
    pub struct StartWith<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        value: Option<S::Item>,
    }
}

impl<S: Stream> StartWith<S> {
    pub(crate) fn new(stream: S, value: S::Item) -> Self {
        Self {
            stream: stream.fuse(),
            value: Some(value),
        }
    }
}

impl<S> FusedStream for StartWith<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for StartWith<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let Some(value) = this.value.take() {
            Poll::Ready(Some(value))
        } else {
            this.stream.as_mut().poll_next(cx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();

        (a + 1, b.map(|it| it + 1))
    }
}
