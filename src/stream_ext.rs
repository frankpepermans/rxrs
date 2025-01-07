use std::{collections::VecDeque, future::Future, hash::Hash, vec::IntoIter};

use buffer::Buffer;
use debounce::Debounce;
use dematerialize::Dematerialize;
use distinct::Distinct;
use distinct_until_changed::DistinctUntilChanged;
use futures::{stream::Iter, Stream};
use materialize::Materialize;
use pairwise::Pairwise;
use race::Race;
use share::Shared;
use start_with::StartWith;
use switch_map::SwitchMap;
use window::Window;

use crate::{BehaviorSubject, CombineLatest2, Event, Notification, PublishSubject, ReplaySubject};

use self::delay::Delay;

pub mod buffer;
pub mod debounce;
pub mod delay;
pub mod dematerialize;
pub mod distinct;
pub mod distinct_until_changed;
pub mod materialize;
pub mod pairwise;
pub mod race;
pub mod share;
pub mod start_with;
pub mod switch_map;
pub mod window;

impl<T: ?Sized> RxExt for T where T: Stream {}
pub trait RxExt: Stream {
    fn race<S: Stream<Item = Self::Item>>(self, other: S) -> Race<Self, S, Self::Item>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Race::new(self, other))
    }

    fn start_with<I: IntoIterator<Item = Self::Item>>(self, iter: I) -> StartWith<Self>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(StartWith::new(self, iter))
    }

    fn share(self) -> Shared<Self, PublishSubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, PublishSubject::new()))
    }

    fn share_behavior(self) -> Shared<Self, BehaviorSubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, BehaviorSubject::new()))
    }

    fn share_replay(self) -> Shared<Self, ReplaySubject<Self::Item>>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, ReplaySubject::new()))
    }

    fn switch_map<S: Stream, F: FnMut(Self::Item) -> S>(self, f: F) -> SwitchMap<Self, S, F>
    where
        Self: Sized,
    {
        assert_stream::<<F::Output as Stream>::Item, _>(SwitchMap::new(self, f))
    }

    fn pairwise(self) -> Pairwise<Self>
    where
        Self: Sized,
    {
        assert_stream::<(Self::Item, Event<Self::Item>), _>(Pairwise::new(self))
    }

    fn debounce<Fut: Future, F: Fn(&Self::Item) -> Fut>(self, f: F) -> Debounce<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Debounce::new(self, f))
    }

    fn buffer<Fut: Future<Output = bool>, F: Fn(&Self::Item, usize) -> Fut>(
        self,
        f: F,
    ) -> Buffer<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<VecDeque<Self::Item>, _>(Buffer::new(self, f))
    }

    fn window<Fut: Future<Output = bool>, F: Fn(&Self::Item, usize) -> Fut>(
        self,
        f: F,
    ) -> Window<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Iter<IntoIter<Self::Item>>, _>(Window::new(self, f))
    }

    fn distinct(self) -> Distinct<Self>
    where
        Self: Sized,
        Self::Item: Hash,
    {
        assert_stream::<Self::Item, _>(Distinct::new(self))
    }

    fn distinct_until_changed(self) -> DistinctUntilChanged<Self>
    where
        Self: Sized,
        Self::Item: Hash,
    {
        assert_stream::<Self::Item, _>(DistinctUntilChanged::new(self))
    }

    fn materialize(self) -> Materialize<Self>
    where
        Self: Sized,
    {
        assert_stream::<Notification<Self::Item>, _>(Materialize::new(self))
    }

    fn dematerialize<T>(self) -> Dematerialize<Self, T>
    where
        Self: Stream<Item = Notification<T>> + Sized,
    {
        assert_stream::<T, _>(Dematerialize::new(self))
    }

    fn delay<Fut: Future, F: Fn() -> Fut>(self, f: F) -> Delay<Self, Fut, F>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Delay::new(self, f))
    }

    fn with_latest_from<S: Stream>(self, stream: S) -> CombineLatest2<Self, S, Self::Item, S::Item>
    where
        Self: Sized,
        Self::Item: ToOwned<Owned = Self::Item>,
        S::Item: ToOwned<Owned = S::Item>,
    {
        assert_stream::<(Self::Item, S::Item), _>(CombineLatest2::new(self, stream))
    }
}

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
