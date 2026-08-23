use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures::stream::Fuse;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;
use pin_project_lite::pin_project;
use std::sync::Arc;
use std::sync::Mutex;

use crate::subject::shareable_subject::ShareableSubject;
use crate::subject::Subject;
use crate::Event;
use crate::Observable;

pin_project! {
    /// Stream for the [`share`](RxStreamExt::share) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Shared<S: Stream, Sub: Subject<Item = S::Item>> {
        inner: Arc<Mutex<ShareableSubject<S, Sub>>>,
        #[pin]
        stream: Fuse<Observable<S::Item>>,
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Shared<S, Sub> {
    pub(crate) fn new(stream: S, subject: Sub) -> Self {
        let mut subject = ShareableSubject::new(stream, subject);
        let stream = subject.subscribe().fuse();

        Self {
            inner: Arc::new(Mutex::new(subject)),
            stream,
        }
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Clone for Shared<S, Sub> {
    fn clone(&self) -> Self {
        let stream = self.inner.lock().unwrap().subscribe().fuse();

        Self {
            inner: Arc::clone(&self.inner),
            stream,
        }
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> Stream for Shared<S, Sub> {
    type Item = Event<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.lock().unwrap().poll_next(cx);
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<S: Stream, Sub: Subject<Item = S::Item>> FusedStream for Shared<S, Sub> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, future::join, stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        block_on(async {
            let stream = stream::iter(1usize..=3usize);
            let s1 = stream.share();
            let s2 = s1.clone();
            let (a, b) = join(s1.collect::<Vec<_>>(), s2.collect::<Vec<_>>()).await;

            assert_eq!(a, [1.into(), 2.into(), 3.into()]);
            assert_eq!(b, [1.into(), 2.into(), 3.into()]);
        });
    }
}

#[cfg(test)]
mod edge_test {
    use futures::{
        executor::block_on,
        future::{join, join3},
        stream, StreamExt,
    };

    use crate::{test_util::stuttering, RxExt};

    #[test]
    fn an_empty_source_completes_every_subscriber() {
        block_on(async {
            let a = stream::empty::<i32>().share();
            let b = a.clone();
            let (a, b) = join(a.collect::<Vec<_>>(), b.collect::<Vec<_>>()).await;

            assert_eq!(a.len(), 0);
            assert_eq!(b.len(), 0);
        });
    }

    #[test]
    fn three_subscribers_all_receive_every_event() {
        block_on(async {
            let a = stream::iter(1..=3).share();
            let (b, c) = (a.clone(), a.clone());
            let (a, b, c) = join3(
                a.map(|it| *it).collect::<Vec<_>>(),
                b.map(|it| *it).collect::<Vec<_>>(),
                c.map(|it| *it).collect::<Vec<_>>(),
            )
            .await;

            assert_eq!(a, [1, 2, 3]);
            assert_eq!(b, [1, 2, 3]);
            assert_eq!(c, [1, 2, 3]);
        });
    }

    #[test]
    fn subscribers_share_one_underlying_value_per_event() {
        block_on(async {
            let mut a = stream::iter([vec![1, 2, 3]]).share();
            let mut b = a.clone();
            let (a, b) = join(a.next(), b.next()).await;

            assert!(std::ptr::eq(
                a.unwrap().borrow_value(),
                b.unwrap().borrow_value()
            ));
        });
    }

    #[test]
    fn the_source_is_only_consumed_once() {
        block_on(async {
            let a = stuttering(1..=4).share();
            let b = a.clone();
            let (a, b) = join(
                a.map(|it| *it).collect::<Vec<_>>(),
                b.map(|it| *it).collect::<Vec<_>>(),
            )
            .await;

            assert_eq!(a, [1, 2, 3, 4]);
            assert_eq!(b, [1, 2, 3, 4]);
        });
    }

    #[test]
    fn a_lone_subscriber_behaves_like_the_source() {
        block_on(async {
            let events = stream::iter(1..=3)
                .share()
                .map(|it| *it)
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [1, 2, 3]);
        });
    }

    #[test]
    fn share_behavior_replays_the_initial_value_to_every_subscriber() {
        block_on(async {
            let a = stream::iter(1..=2).share_behavior(0);
            let b = a.clone();
            let (a, b) = join(
                a.map(|it| *it).collect::<Vec<_>>(),
                b.map(|it| *it).collect::<Vec<_>>(),
            )
            .await;

            assert_eq!(a, [0, 1, 2]);
            assert_eq!(b, [0, 1, 2]);
        });
    }

    #[test]
    fn share_replay_replays_everything_seen_so_far() {
        block_on(async {
            let mut a = stream::iter(1..=3).share_replay();

            assert_eq!(a.next().await.map(|it| *it), Some(1));

            let b = a.clone();
            let (a, b) = join(
                a.map(|it| *it).collect::<Vec<_>>(),
                b.map(|it| *it).collect::<Vec<_>>(),
            )
            .await;

            assert_eq!(a, [2, 3]);
            assert_eq!(b, [1, 2, 3]);
        });
    }
}
