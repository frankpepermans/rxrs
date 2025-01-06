use std::{collections::VecDeque, future::Future};

use buffer::Buffer;
use debounce::Debounce;
use futures::Stream;
use into_ref_steam::IntoRefStream;
use pairwise::Pairwise;
use race::Race;
use share::Shared;
use start_with::StartWith;
use switch_map::SwitchMap;

use crate::{BehaviorSubject, Event, PublishSubject, ReplaySubject};

pub mod buffer;
pub mod debounce;
pub mod into_ref_steam;
pub mod pairwise;
pub mod race;
pub mod share;
pub mod start_with;
pub mod switch_map;

impl<T: ?Sized> RxExt for T where T: Stream {}
pub trait RxExt: Stream {
    fn into_ref_stream(self) -> IntoRefStream<Self>
    where
        Self: Sized,
    {
        assert_stream::<Event<Self::Item>, _>(IntoRefStream::new(self))
    }

    fn race<S: Stream<Item = Self::Item>>(self, other: S) -> Race<Self, S, Self::Item>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Race::new(self, other))
    }

    fn start_with(self, value: Self::Item) -> StartWith<Self>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(StartWith::new(self, value))
    }

    fn share(self) -> Shared<Self, PublishSubject<Self::Item>>
    where
        Self: Sized + Unpin,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, PublishSubject::new()))
    }

    fn share_behavior(self) -> Shared<Self, BehaviorSubject<Self::Item>>
    where
        Self: Sized + Unpin,
    {
        assert_stream::<Event<Self::Item>, _>(Shared::new(self, BehaviorSubject::new()))
    }

    fn share_replay(self) -> Shared<Self, ReplaySubject<Self::Item>>
    where
        Self: Sized + Unpin,
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
        assert_stream::<(Event<Self::Item>, Event<Self::Item>), _>(Pairwise::new(self))
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
}

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
