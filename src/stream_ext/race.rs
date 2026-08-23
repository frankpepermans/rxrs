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
    pub struct Race<S1: Stream, S2: Stream<Item = S1::Item>> {
        #[pin]
        left: Fuse<S1>,
        #[pin]
        right: Fuse<S2>,
        winner: Winner,
    }
}

impl<S1: Stream, S2: Stream<Item = S1::Item>> Race<S1, S2> {
    pub(crate) fn new(left: S1, right: S2) -> Self {
        Self {
            left: left.fuse(),
            right: right.fuse(),
            winner: Winner::Undecided,
        }
    }
}

impl<S1, S2> FusedStream for Race<S1, S2>
where
    S1: FusedStream,
    S2: FusedStream<Item = S1::Item>,
{
    fn is_terminated(&self) -> bool {
        match &self.winner {
            Winner::Left => self.left.is_terminated(),
            Winner::Right => self.right.is_terminated(),
            Winner::Undecided => self.left.is_terminated() && self.right.is_terminated(),
        }
    }
}

impl<S1: Stream, S2: Stream<Item = S1::Item>> Stream for Race<S1, S2> {
    type Item = S1::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match &this.winner {
            Winner::Left => this.left.poll_next(cx),
            Winner::Right => this.right.poll_next(cx),
            Winner::Undecided => {
                let left = this.left.poll_next(cx);
                let right = this.right.poll_next(cx);

                match (left, right) {
                    (Poll::Ready(Some(it)), _) => {
                        *this.winner = Winner::Left;

                        Poll::Ready(Some(it))
                    }
                    (_, Poll::Ready(Some(it))) => {
                        *this.winner = Winner::Right;

                        Poll::Ready(Some(it))
                    }
                    (Poll::Ready(None), Poll::Ready(None)) => Poll::Ready(None),
                    _ => Poll::Pending,
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

    #[test]
    fn completing_without_emitting_does_not_win() {
        block_on(async {
            let all_events = stream::empty::<&str>()
                .race(stream::iter(["value"]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, ["value"]);

            let all_events = stream::iter(["value"])
                .race(stream::empty::<&str>())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, ["value"]);

            let all_events = stream::empty::<&str>()
                .race(stream::empty::<&str>())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [] as [&str; 0]);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{
        test_util::{never, stuttering},
        RxExt,
    };

    #[test]
    fn the_first_stream_to_emit_wins_every_later_event() {
        block_on(async {
            let events = stream::iter([1, 2, 3])
                .race(stream::iter([4, 5, 6]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2, 3]);
        });
    }

    #[test]
    fn the_loser_is_never_emitted_even_when_it_becomes_ready() {
        block_on(async {
            let events = stuttering([1, 2])
                .race(stream::iter([9]))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [9]);
        });
    }

    #[test]
    fn a_stream_that_never_emits_never_wins() {
        block_on(async {
            let events = never().race(stuttering([1, 2])).collect::<Vec<_>>().await;

            assert_eq!(events, [1, 2]);
        });
    }

    #[test]
    fn racing_two_empty_streams_completes_empty() {
        block_on(async {
            let events = stream::empty::<i32>()
                .race(stream::empty())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }
}
