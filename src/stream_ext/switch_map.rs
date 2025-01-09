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
    pub struct SwitchMap<S: Stream, St: Stream, F: FnMut(S::Item) -> St> {
        #[pin]
        stream: Fuse<S>,
        #[pin]
        switch_stream: Option<Fuse<F::Output>>,
        f: F,
    }
}

impl<S: Stream, St: Stream, F: FnMut(S::Item) -> St> SwitchMap<S, St, F> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream: stream.fuse(),
            switch_stream: None,
            f,
        }
    }
}

impl<S: Stream, St: Stream, F: FnMut(S::Item) -> St> FusedStream for SwitchMap<S, St, F> {
    fn is_terminated(&self) -> bool {
        if self.stream.is_terminated() {
            self.switch_stream
                .as_ref()
                .map(|it| it.is_terminated())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

impl<S: Stream, St: Stream, F: FnMut(S::Item) -> St> Stream for SwitchMap<S, St, F>
where
    F::Output: Stream,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let is_done = match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                this.switch_stream.set((this.f)(event).fuse().into());

                false
            }
            Poll::Ready(None) => true,
            Poll::Pending => false,
        };

        this.switch_stream
            .as_pin_mut()
            .map(|it| it.poll_next(cx))
            .unwrap_or_else(|| {
                if is_done {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.stream.is_terminated() {
            self.switch_stream
                .as_ref()
                .map(|it| it.size_hint())
                .unwrap_or((0, None))
        } else {
            let (lower, _) = self.stream.size_hint();

            (lower, None)
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(0usize..=3usize);
            let all_events = stream
                .switch_map(|i| stream::iter([i.pow(2), i.pow(3), i.pow(4)]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [0, 1, 4, 9, 27, 81]);
        });
    }
}
