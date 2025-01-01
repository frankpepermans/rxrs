use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::{Event, EventStream};

pin_project! {
    /// Stream for the [`into_ref_stream`](RxStreamExt::into_ref_stream) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct IntoRefStream<S: Stream> {
        #[pin]
        stream: EventStream<S::Item, S>,
    }
}

impl<S: Stream> IntoRefStream<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream: EventStream::new(stream),
        }
    }
}

impl<S> FusedStream for IntoRefStream<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_done()
    }
}

impl<S> Stream for IntoRefStream<S>
where
    S: Stream,
{
    type Item = Event<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.as_mut().poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
