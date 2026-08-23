use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures::{stream::FusedStream, Stream};

use crate::{Controller, Event};

pub struct Observable<T> {
    inner: Arc<RwLock<Controller<Event<T>>>>,
}

impl<T> Observable<T> {
    pub(crate) fn new(inner: Arc<RwLock<Controller<Event<T>>>>) -> Self {
        Self { inner }
    }
}

impl<T> FusedStream for Observable<T> {
    fn is_terminated(&self) -> bool {
        self.inner.read().unwrap().is_done
    }
}

impl<T> Stream for Observable<T> {
    type Item = Event<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_mut().inner.write().unwrap().pop(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let inner = self.inner.read().unwrap();
        let lower_bound = inner.len();
        let upper_bound = if inner.is_done {
            Some(lower_bound)
        } else {
            None
        };

        (lower_bound, upper_bound)
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures::{executor::block_on, future::join, StreamExt};

    use crate::{PublishSubject, Subject};

    use super::*;

    #[test]
    fn emits_every_event_a_subject_publishes() {
        block_on(async {
            let mut subject = PublishSubject::new();
            let observable = subject.subscribe();

            subject.next(1);
            subject.next(2);
            subject.close();

            let events = observable.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events, [1, 2]);
        });
    }

    #[test]
    fn completes_once_its_subject_is_dropped() {
        block_on(async {
            let mut subject = PublishSubject::new();
            let observable = subject.subscribe();

            subject.next(1);
            drop(subject);

            let events = observable.map(|it| *it).collect::<Vec<_>>().await;

            assert_eq!(events, [1]);
        });
    }

    #[test]
    fn reports_termination_only_once_closed() {
        let mut subject = PublishSubject::<i32>::new();
        let observable = subject.subscribe();

        assert!(!observable.is_terminated());

        subject.close();

        assert!(observable.is_terminated());
    }

    #[test]
    fn size_hint_is_bounded_only_once_closed() {
        let mut subject = PublishSubject::new();
        let observable = subject.subscribe();

        subject.next(1);
        subject.next(2);

        assert_eq!(observable.size_hint(), (2, None));

        subject.close();

        assert_eq!(observable.size_hint(), (2, Some(2)));
    }

    /// An idle `Observable` must park on its waker rather than spin, so the
    /// number of polls stays proportional to the events, not to the wait.
    #[test]
    fn parks_while_idle_instead_of_spinning() {
        static POLLS: AtomicUsize = AtomicUsize::new(0);

        struct Counting<S>(S);

        impl<S: Stream + Unpin> Stream for Counting<S> {
            type Item = S::Item;

            fn poll_next(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                POLLS.fetch_add(1, Ordering::Relaxed);

                Pin::new(&mut self.0).poll_next(cx)
            }
        }

        block_on(async {
            let mut subject = PublishSubject::new();
            let observable = Counting(subject.subscribe());
            let feed = async {
                futures_time::future::IntoFuture::into_future(
                    futures_time::time::Duration::from_millis(50),
                )
                .await;

                subject.next(1);
                subject.close();
            };
            let (events, _) = join(observable.map(|it| *it).collect::<Vec<_>>(), feed).await;

            assert_eq!(events, [1]);
            assert!(
                POLLS.load(Ordering::Relaxed) < 20,
                "observable spun {} times while idle",
                POLLS.load(Ordering::Relaxed)
            );
        });
    }
}
