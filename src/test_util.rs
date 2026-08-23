use std::task::Poll;

use futures::{stream, Stream};

/// A stream that yields `Poll::Pending` once before every event, so that
/// adapters are driven through their pending paths instead of only their
/// always-ready ones.
pub(crate) fn stuttering<T>(items: impl IntoIterator<Item = T>) -> impl Stream<Item = T> + Unpin {
    let mut items = items.into_iter();
    let mut is_pending = false;

    Box::pin(stream::poll_fn(move |cx| {
        is_pending = !is_pending;

        if is_pending {
            cx.waker().wake_by_ref();

            Poll::Pending
        } else {
            Poll::Ready(items.next())
        }
    }))
}

/// A stream that never yields an event and never completes.
pub(crate) fn never<T>() -> impl Stream<Item = T> + Unpin {
    Box::pin(stream::poll_fn(|_| Poll::Pending))
}
