use std::{collections::VecDeque, future::Future, hash::Hash, vec::IntoIter};

use buffer::Buffer;
use debounce::Debounce;
use delay_every::DelayEvery;
use dematerialize::Dematerialize;
use distinct::Distinct;
use distinct_until_changed::DistinctUntilChanged;
use futures::{stream::Iter, Stream};
use inspect_done::InspectDone;
use materialize::Materialize;
use pairwise::Pairwise;
use race::Race;
use sample::Sample;
use share::Shared;
use start_with::StartWith;
use switch_map::SwitchMap;
use timing::{Timed, Timing};
use window::Window;

use crate::{
    BehaviorSubject, CombineLatest2, Event, EventLite, Notification, PublishSubject, ReplaySubject,
};

use self::{delay::Delay, end_with::EndWith, throttle::Throttle};

pub mod buffer;
pub mod debounce;
pub mod delay;
pub mod delay_every;
pub mod dematerialize;
pub mod distinct;
pub mod distinct_until_changed;
pub mod end_with;
pub mod inspect_done;
pub mod materialize;
pub mod pairwise;
pub mod race;
pub mod sample;
pub mod share;
pub mod start_with;
pub mod switch_map;
pub mod throttle;
pub mod timing;
pub mod window;

impl<T: ?Sized> RxExt for T where T: Stream {}
pub trait RxExt: Stream {
    /// Starts polling itself as well as the provided other `Stream`.
    /// The first one to emit an event "wins" and proceeds to emit all its next events.
    /// The "loser" is discarded and will not be polled further.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let slower_stream = stream::iter(4..=6).delay(|| async { /* return delayed over time */ });
    /// let stream = stream.race(slower_stream);
    ///
    /// assert_eq!(vec![0, 1, 2, 3], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn race<S: Stream<Item = Self::Item>>(self, other: S) -> Race<Self, S, Self::Item>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Race::new(self, other))
    }

    /// Precedes all emitted events with the items of an iter.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(4..=6);
    /// let stream = stream.start_with(0..=3);
    ///
    /// assert_eq!(vec![0, 1, 2, 3, 4, 5, 6], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn start_with<I: IntoIterator<Item = Self::Item>>(self, iter: I) -> StartWith<Self>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(StartWith::new(self, iter))
    }

    /// Follows all emitted events with the items of an iter.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.end_with(4..=6);
    ///
    /// assert_eq!(vec![0, 1, 2, 3, 4, 5, 6], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn end_with<I: IntoIterator<Item = Self::Item>>(self, iter: I) -> EndWith<Self>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(EndWith::new(self, iter))
    }

    /// Transforms a `Stream` into a broadcast one, which can be subscribed to more than once, after cloning the shared version.
    ///
    /// Behavior is exactly like a `PublishSubject`, every new subscription will produce a unique `Stream` which only emits `Event` objects.
    /// An `Event` is a helper object which wraps a ref counted value.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::{stream::{StreamExt, self}, future::join};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.share();
    /// let sub_stream_a = stream.clone().map(|event| *event); // an event is Event here, wrapping a ref counted value
    /// let sub_stream_b = stream.clone().map(|event| *event); // which we can just deref in this case to i32
    ///
    /// assert_eq!((vec![0, 1, 2, 3], vec![0, 1, 2, 3]), join(sub_stream_a.collect::<Vec<_>>(), sub_stream_b.collect::<Vec<_>>()).await);
    /// # });
    ///
    /// #
    /// ```
    fn share(self) -> Shared<Self, PublishSubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, PublishSubject::new()))
    }

    /// Transforms a `Stream` into a broadcast one, which can be subscribed to more than once, after cloning the shared version.
    ///
    /// Behavior is exactly like a `BehaviorSubject`, where every new subscription will always receive the last emitted event
    /// from the parent `Stream` first.
    /// Every new subscription will produce a unique `Stream` which only emits `Event` objects.
    /// An `Event` is a helper object which wraps a ref counted value.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::{stream::{StreamExt, self}, future::join};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(1..=3);
    /// let stream = stream.share_behavior(0);
    ///
    /// stream.clone().collect::<Vec<_>>().await; // consume all events beforehand
    ///
    /// let sub_stream_a = stream.clone().map(|event| *event); // an event is Event here, wrapping a ref counted value
    /// let sub_stream_b = stream.clone().map(|event| *event); // which we can just deref in this case to i32
    ///
    /// assert_eq!(
    ///     (vec![3], vec![3]),
    ///     join(
    ///         sub_stream_a.collect::<Vec<_>>(),
    ///         sub_stream_b.collect::<Vec<_>>()
    ///     )
    ///     .await
    /// );
    /// # });
    ///
    /// #
    /// ```
    fn share_behavior(self, initial_value: Self::Item) -> Shared<Self, BehaviorSubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(
            self,
            BehaviorSubject::new(initial_value),
        ))
    }

    /// Transforms a `Stream` into a broadcast one, which can be subscribed to more than once, after cloning the shared version.
    ///
    /// Behavior is exactly like a `ReplaySubject`, where every new subscription will always receive all previously emitted events
    /// from the parent `Stream` first.
    /// Every new subscription will produce a unique `Stream` which only emits `Event` objects.
    /// An `Event` is a helper object which wraps a ref counted value.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::{stream::{StreamExt, self}, future::join};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.share_replay();
    ///
    /// stream.clone().collect::<Vec<_>>().await; // consume all events beforehand
    ///
    /// let sub_stream_a = stream.clone().map(|event| *event); // an event is Event here, wrapping a ref counted value
    /// let sub_stream_b = stream.clone().map(|event| *event); // which we can just deref in this case to i32
    ///
    /// assert_eq!(
    ///     (vec![0, 1, 2, 3], vec![0, 1, 2, 3]),
    ///     join(
    ///         sub_stream_a.collect::<Vec<_>>(),
    ///         sub_stream_b.collect::<Vec<_>>()
    ///     )
    ///     .await
    /// );
    /// # });
    ///
    /// #
    /// ```
    fn share_replay(self) -> Shared<Self, ReplaySubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, ReplaySubject::new()))
    }

    /// Like `flat_map`, except that switched `Stream` is interrupted when the parent `Stream` emits a next event.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.switch_map(|event| stream::iter([event + 10, event - 10]));
    ///
    /// assert_eq!(vec![10, 11, 12, 13, -7], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn switch_map<S: Stream, F: FnMut(Self::Item) -> S>(self, f: F) -> SwitchMap<Self, S, F>
    where
        Self: Sized,
    {
        assert_stream::<<F::Output as Stream>::Item, _>(SwitchMap::new(self, f))
    }

    /// Emits pairs of the previous and next events as a tuple.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// The next value in the tuple is a value reference, and therefore wrapped inside an `Event` struct.
    /// An `Event` is a helper object for ref counted events.
    /// As the next event will also need to be emitted as the previous event in the next pair,
    /// it is first made available as next using a ref count - `Event`.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.pairwise();
    /// let stream = stream.map(|(prev, next)| (prev, *next)); // we can deref here to i32
    ///
    /// assert_eq!(vec![(0, 1), (1, 2), (2, 3)], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn pairwise(self) -> Pairwise<Self>
    where
        Self: Sized,
    {
        assert_stream::<(Self::Item, EventLite<Self::Item>), _>(Pairwise::new(self))
    }

    /// Delays events using a debounce time window.
    /// The event will emit when this window closes and when no other event
    /// was emitted while this window was open.
    ///
    /// The provided closure is executed over all elements of this stream as
    /// they are made available. It is executed inline with calls to
    /// [`poll_next`](Stream::poll_next).
    ///
    /// The debounce window resets on every newly emitted event.
    /// On next, the closure is invoked and a reference to the event is passed.
    /// The closure needs to return a `Future`, which represents the next debounce window over time.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    fn debounce<Fut: Future, F: FnMut(&Self::Item) -> Fut>(self, f: F) -> Debounce<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Debounce::new(self, f))
    }

    /// Creates a new interval from the closure, whenever a new event is emitted from the parent `Stream`.
    /// This event is immediately emitted, however for as long as the interval is now open, no
    /// subsequent events will be emitted.
    ///
    /// When the interval closes and the parent `Stream` emits a new event, this
    /// process repeats.
    ///
    /// The provided closure is executed over all elements of this stream as
    /// they are made available. It is executed inline with calls to
    /// [`poll_next`](Stream::poll_next).
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// See also `sample`
    fn throttle<Fut: Future, F: FnMut(&Self::Item) -> Fut>(self, f: F) -> Throttle<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Throttle::new(self, f, throttle::ThrottleConfig::Leading))
    }

    /// Like `throttle`, but only emitting trailing items.
    fn throttle_trailing<Fut: Future, F: FnMut(&Self::Item) -> Fut>(
        self,
        f: F,
    ) -> Throttle<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Throttle::new(self, f, throttle::ThrottleConfig::Trailing))
    }

    /// Like `throttle`, but emitting both leading and trailing items.
    fn throttle_all<Fut: Future, F: FnMut(&Self::Item) -> Fut>(self, f: F) -> Throttle<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Throttle::new(self, f, throttle::ThrottleConfig::All))
    }

    /// Creates chunks of buffered data.
    ///
    /// The provided closure is executed over all elements of this stream as
    /// they are made available. It is executed inline with calls to
    /// [`poll_next`](Stream::poll_next).
    ///
    /// You can use a reference to the current event, or the count of the current buffer
    /// to determine when a chunk should close and emit next.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use std::collections::VecDeque;
    ///
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..9);
    /// let stream = stream.buffer(|_, count| async move { count == 3 });
    ///
    /// assert_eq!(
    ///     vec![
    ///         VecDeque::from_iter([0, 1, 2]),
    ///         VecDeque::from_iter([3, 4, 5]),
    ///         VecDeque::from_iter([6, 7, 8])
    ///     ],
    ///     stream.collect::<Vec<_>>().await
    /// );
    /// # });
    ///
    /// #
    /// ```
    fn buffer<Fut: Future<Output = bool>, F: FnMut(&Self::Item, usize) -> Fut>(
        self,
        f: F,
    ) -> Buffer<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<VecDeque<Self::Item>, _>(Buffer::new(self, f))
    }

    /// Creates chunks of buffered data as new `Stream`s.
    ///
    /// The provided closure is executed over all elements of this stream as
    /// they are made available. It is executed inline with calls to
    /// [`poll_next`](Stream::poll_next).
    ///
    /// You can use a reference to the current event, or the count of the current buffer
    /// to determine when a chunk should close and emit next.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..9);
    /// let stream = stream.window(|_, count| async move { count == 3 }).flat_map(|it| it);
    ///
    /// assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn window<Fut: Future<Output = bool>, F: FnMut(&Self::Item, usize) -> Fut>(
        self,
        f: F,
    ) -> Window<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Iter<IntoIter<Self::Item>>, _>(Window::new(self, f))
    }

    /// Ensures that all emitted events are unique.
    /// Events are required to implement `Hash`.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter([1, 2, 1, 3, 2, 2, 1, 4]);
    /// let stream = stream.distinct();
    ///
    /// assert_eq!(vec![1, 2, 3, 4], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn distinct(self) -> Distinct<Self>
    where
        Self: Sized,
        Self::Item: Hash,
    {
        assert_stream::<Self::Item, _>(Distinct::new(self))
    }

    /// Ensures that all emitted events are unique within immediate sequence.
    /// Events are required to implement `Hash`.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter([1, 1, 1, 2, 2, 2, 3, 1, 1]);
    /// let stream = stream.distinct_until_changed();
    ///
    /// assert_eq!(vec![1, 2, 3, 1], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn distinct_until_changed(self) -> DistinctUntilChanged<Self>
    where
        Self: Sized,
        Self::Item: Hash,
    {
        assert_stream::<Self::Item, _>(DistinctUntilChanged::new(self))
    }

    /// Converts all events of a `Stream` into `Notification` events.
    /// When the `Stream` is done, it will first emit a final `Notification::Complete` event.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::{Notification, RxExt};
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.materialize();
    ///
    /// assert_eq!(
    ///     vec![
    ///         Notification::Next(0),
    ///         Notification::Next(1),
    ///         Notification::Next(2),
    ///         Notification::Next(3),
    ///         Notification::Complete
    ///     ],
    ///     stream.collect::<Vec<_>>().await
    /// );
    /// # });
    ///
    /// #
    /// ```
    fn materialize(self) -> Materialize<Self>
    where
        Self: Sized,
    {
        assert_stream::<Notification<Self::Item>, _>(Materialize::new(self))
    }

    /// The inverse of materialize.
    /// Use this transformer to translate a `Stream` emitting `Notification` events back
    /// into a `Stream` emitting original events.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.materialize().dematerialize();
    ///
    /// assert_eq!(vec![0, 1, 2, 3], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn dematerialize<T>(self) -> Dematerialize<Self, T>
    where
        Self: Stream<Item = Notification<T>> + Sized,
    {
        assert_stream::<T, _>(Dematerialize::new(self))
    }

    /// Delays emitting events using an initial time window, provided by a closure.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.delay(|| async { /* return delayed over time */ });
    ///
    /// assert_eq!(vec![0, 1, 2, 3], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn delay<Fut: Future, F: FnMut() -> Fut>(self, f: F) -> Delay<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Delay::new(self, f))
    }

    /// Delays every event using a time window, provided by a closure.
    ///
    /// Use max_buffer_size to limit the amount of buffered items that are awaiting
    /// time window(s) to complete.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.delay_every(|_| async { /* return delayed over time */ }, None);
    ///
    /// assert_eq!(vec![0, 1, 2, 3], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn delay_every<Fut: Future, F: FnMut(&Self::Item) -> Fut>(
        self,
        f: F,
        max_buffer_size: Option<usize>,
    ) -> DelayEvery<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(DelayEvery::new(self, f, max_buffer_size))
    }

    /// Acts just like a `CombineLatest2`, where every next event is a tuple pair
    /// containing the last emitted events from both `Stream`s.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let stream = stream::iter(0..=3);
    /// let stream = stream.with_latest_from(stream::iter(0..=3));
    ///
    /// assert_eq!(vec![(0, 0), (1, 1), (2, 2), (3, 3)], stream.collect::<Vec<_>>().await);
    /// # });
    ///
    /// #
    /// ```
    fn with_latest_from<S: Stream>(self, stream: S) -> CombineLatest2<Self, S, Self::Item, S::Item>
    where
        Self: Sized,
        Self::Item: ToOwned<Owned = Self::Item>,
        S::Item: ToOwned<Owned = S::Item>,
    {
        assert_stream::<(Self::Item, S::Item), _>(CombineLatest2::new(self, stream))
    }

    /// Wraps each item into a `Timed` struct.
    /// This structs hold the actual event, as well as
    /// a timestamp containing an `Instant` and an elapsed interval
    /// as `Duration`, relative to the second to last emitted event.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    fn timing(self) -> Timing<Self>
    where
        Self: Sized,
    {
        assert_stream::<Timed<Self::Item>, _>(Timing::new(self))
    }

    /// Similar to `inspect`, except that the closure provided is only ever
    /// triggered when the `Stream` is done.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use futures_rx::RxExt;
    ///
    /// let mut is_done = false;
    /// stream::iter(0..=8)
    ///     .inspect_done(|| is_done = true)
    ///     .collect::<Vec<_>>()
    ///     .await;
    ///
    /// assert!(is_done);
    /// # });
    ///
    /// #
    /// ```
    fn inspect_done<F: FnMut()>(self, f: F) -> InspectDone<Self, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(InspectDone::new(self, f))
    }

    /// Only emits events whenever the `sampler` emits an event.
    /// The event emitted is then the last emitted event from the
    /// source `Stream`.
    ///
    /// If the `sampler` triggers before the source `Stream` was
    /// able to produce a new event, then no event is emitted.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// See also `throttle`.
    fn sample<S: Stream>(self, sampler: S) -> Sample<Self, S>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Sample::new(self, sampler))
    }
}

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
