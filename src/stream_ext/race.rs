use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

enum Winner {
    Left,
    Right,
    Undecided,
}

pin_project! {
    /// Stream for the [`race`](RxStreamExt::race) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Race<S1: Stream<Item = T>, S2: Stream<Item = T>, T> {
        #[pin]
        left: Fuse<S1>,
        #[pin]
        right: Fuse<S2>,
        winner: Winner,
    }
}

impl<S1: Stream<Item = T>, S2: Stream<Item = T>, T> Race<S1, S2, T> {
    pub(crate) fn new(left: S1, right: S2) -> Self {
        Self {
            left: left.fuse(),
            right: right.fuse(),
            winner: Winner::Undecided,
        }
    }
}

impl<S1: Stream<Item = T>, S2: Stream<Item = T>, T> FusedStream for Race<S1, S2, T>
where
    S1: FusedStream,
    S2: FusedStream,
{
    fn is_terminated(&self) -> bool {
        match &self.winner {
            Winner::Left => self.left.is_terminated(),
            Winner::Right => self.right.is_terminated(),
            Winner::Undecided => self.left.is_terminated() && self.right.is_terminated(),
        }
    }
}

impl<S1: Stream<Item = T>, S2: Stream<Item = T>, T> Stream for Race<S1, S2, T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match &this.winner {
            Winner::Left => this.left.poll_next(cx),
            Winner::Right => this.right.poll_next(cx),
            Winner::Undecided => {
                let left = this.left.poll_next(cx);
                let right = this.right.poll_next(cx);

                if left.is_ready() {
                    *this.winner = Winner::Left;
                    left
                } else if right.is_ready() {
                    *this.winner = Winner::Right;
                    right
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower_left, upper_left) = self.left.size_hint();
        let (lower_right, upper_right) = self.right.size_hint();

        match &self.winner {
            Winner::Left => (lower_left, upper_left),
            Winner::Right => (lower_right, upper_right),
            Winner::Undecided => (lower_left.min(lower_right), None),
        }
    }
}

#[cfg(test)]
mod test {
    use std::task::Poll;

    use futures::{executor::block_on, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let mut phase = 0usize;
            let fast_stream = stream::iter(["fast"]);
            let slow_stream = stream::poll_fn(move |_| {
                phase += 1;

                match phase {
                    1 => Poll::Pending,
                    2 => Poll::Ready(Some("slow")),
                    3 => Poll::Ready(None),
                    _ => unreachable!(),
                }
            });
            let all_events = slow_stream.race(fast_stream).collect::<Vec<_>>().await;

            assert_eq!(all_events, ["fast"]);
        });
    }
}
