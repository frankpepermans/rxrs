use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{ready, stream::FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::events::{Event, EventStream};

macro_rules! delegate_access_inner {
    ($field:ident, $inner:ty, ($($ind:tt)*)) => {
        /// Acquires a reference to the underlying sink or stream that this combinator is
        /// pulling from.
        pub fn get_ref(&self) -> &$inner {
            (&self.$field) $($ind get_ref())*
        }

        /// Acquires a mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        pub fn get_mut(&mut self) -> &mut $inner {
            (&mut self.$field) $($ind get_mut())*
        }

        /// Acquires a pinned mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut $inner> {
            self.project().$field $($ind get_pin_mut())*
        }

        /// Consumes this combinator, returning the underlying sink or stream.
        ///
        /// Note that this may discard intermediate state of this combinator, so
        /// care should be taken to avoid losing resources when this is called.
        pub fn into_inner(self) -> $inner {
            self.$field $($ind into_inner())*
        }
    }
}

impl<T: ?Sized> RxStreamExt for T where T: Stream {}
pub trait RxStreamExt: Stream {
    fn into_ref_stream(self) -> IntoRefStream<Self>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(IntoRefStream::new(self))
    }
}

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

    delegate_access_inner!(stream, EventStream<S::Item, S>, ());
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
        let mut this = self.project();
        let res = ready!(this.stream.as_mut().poll_next(cx));
        Poll::Ready(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
