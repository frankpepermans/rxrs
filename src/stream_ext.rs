use std::time::Duration;

use debounce::Debounce;
use futures::Stream;
use into_ref_steam::IntoRefStream;
use pairwise::Pairwise;
use race::Race;
use share::Shared;
use start_with::StartWith;
use switch_map::SwitchMap;

use crate::{BehaviorSubject, Event, PublishSubject, ReplaySubject};

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

    fn debounce(self, duration: Duration) -> Debounce<Self>
    where
        Self: Sized,
    {
        assert_stream::<Self::Item, _>(Debounce::new(self, duration))
    }
}

#[macro_export]
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

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
