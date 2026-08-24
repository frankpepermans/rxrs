//! Rx implementations for the [`futures`] crate.
//!
//! Every operator is an extension method on [`Stream`](futures::Stream), provided by the
//! [`RxExt`] trait, so they compose with the combinators `futures` already gives you.
//!
//! # Getting started
//!
//! Import the [`prelude`] to bring the operators, the subjects and the event types
//! into scope in one go.
//!
//! ```
//! use futures::{executor::block_on, stream::{self, StreamExt}};
//! use futures_rx::prelude::*;
//!
//! block_on(async {
//!     let events = stream::iter([1, 1, 2, 2, 3])
//!         .distinct_until_changed()
//!         .collect::<Vec<_>>()
//!         .await;
//!
//!     assert_eq!(events, [1, 2, 3]);
//! });
//! ```
//!
//! # Subjects
//!
//! A [`Subject`] is a hand-driven source that fans every event out to each of its
//! subscribers. Subscribing yields an [`Observable`], which is itself a `Stream`, so
//! the operators apply to it too.
//!
//! ```
//! use futures::{executor::block_on, StreamExt};
//! use futures_rx::prelude::*;
//!
//! let mut subject = PublishSubject::new();
//! let first: Observable<i32> = subject.subscribe();
//! let second = subject.subscribe();
//!
//! subject.next(1);
//! subject.next(2);
//! subject.close();
//!
//! block_on(async {
//!     assert_eq!(first.map(|it| *it).collect::<Vec<_>>().await, [1, 2]);
//!     assert_eq!(second.map(|it| *it).collect::<Vec<_>>().await, [1, 2]);
//! });
//! ```
//!
//! Subscribers receive an [`Event`], a handle to a ref counted value, so one event can
//! be shared by every subscriber without requiring `Clone`.
//!
//! See [`RxExt`] for the full list of operators.

pub mod stream;
pub mod stream_ext;
pub mod subject;

#[cfg(test)]
pub(crate) mod test_util;

/// Everything needed to build and consume Rx streams.
///
/// ```
/// use futures_rx::prelude::*;
/// ```
///
/// The crate root re-exports the same set, so `futures_rx::PublishSubject` and
/// `futures_rx::prelude::PublishSubject` are the same type.
pub mod prelude {
    pub use crate::{
        stream::controller::*,
        stream::event::*,
        stream::event_lite::*,
        stream::notification::*,
        stream::observable::*,
        stream::rx::combine_latest::*,
        stream::rx::zip::*,
        stream_ext::RxExt,
        subject::{
            Subject,
            {behavior_subject::*, publish_subject::*, replay_subject::*},
        },
    };
}

pub use crate::prelude::*;
