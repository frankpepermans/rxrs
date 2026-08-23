use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use paste::paste;
use pin_project_lite::pin_project;
use std::cmp::Ordering;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! zip {
    ($name:ident; $($stream:ident),+; $($type:ident),+) => {
        paste! {
            pin_project! {
                pub struct $name<$($stream: Stream<Item = $type>),+, $($type),+> {
                    $(
                        #[pin]
                        [<$stream:lower>]: Fuse<$stream>,
                        [<$type:lower>]: Option<$type>,
                    )+
                }
            }
        }

        impl<$($stream: Stream<Item = $type>),+, $($type),+> $name<$($stream),+, $($type),+> {
            paste! {
                #[allow(clippy::too_many_arguments)]
                pub fn new($(
                    [<$stream:lower>]: $stream),+
                ) -> Self {
                    $name {
                        $(
                            [<$stream:lower>]: [<$stream:lower>].fuse(),
                            [<$type:lower>]: None,
                        )+
                    }
                }
            }
        }

        impl<$($stream: Stream<Item = $type>),+, $($type),+> FusedStream for $name<$($stream),+, $($type),+>
        {
            fn is_terminated(&self) -> bool {
                paste! {
                    $(
                        self.[<$stream:lower>].is_terminated()
                    )&&+
                }
            }
        }

        impl<$($stream: Stream<Item = $type>),+, $($type),+> Stream for $name<$($stream),+, $($type),+>
        {
            type Item = ($($type),+);

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let mut this = self.project();

                fn poll_next<S: Stream<Item = T>, T>(
                    stream: Pin<&mut Fuse<S>>,
                    cx: &mut Context<'_>,
                ) -> Option<T> {
                    match stream.poll_next(cx) {
                        Poll::Ready(Some(it)) => Some(it),
                        _ => None,
                    }
                }

                paste! {
                    $(
                        if !this.[<$stream:lower>].is_terminated() && this.[<$type:lower>].is_none() {
                            *this.[<$type:lower>] = poll_next(this.[<$stream:lower>].as_mut(), cx);
                        };
                    )+

                    if $(this.[<$type:lower>].is_some())&&+ {
                        Poll::Ready(Some((
                            $(
                                this.[<$type:lower>].take().unwrap()
                            ),+
                        )))
                    } else if $(this.[<$stream:lower>].is_terminated())||+ {
                        Poll::Ready(None)
                    } else {
                        Poll::Pending
                    }
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                paste! {
                    let size_hint_all = [$(self.[<$stream:lower>].size_hint()),+];
                }

                (
                    size_hint_all
                        .iter()
                        .min_by(|a, b| a.0.cmp(&b.0))
                        .unwrap().0,
                    size_hint_all
                        .into_iter()
                        .min_by(|a, b| match (a.1, b.1) {
                            (None, None) => Ordering::Equal,
                            (None, Some(_)) => Ordering::Greater,
                            (Some(_), None) => Ordering::Less,
                            (Some(a), Some(b)) => a.cmp(&b),
                    }).unwrap().1
                )
            }
        }
    };
}

zip!(Zip2;S1,S2;T1,T2);
zip!(Zip3;S1,S2,S3;T1,T2,T3);
zip!(Zip4;S1,S2,S3,S4;T1,T2,T3,T4);
zip!(Zip5;S1,S2,S3,S4,S5;T1,T2,T3,T4,T5);
zip!(Zip6;S1,S2,S3,S4,S5,S6;T1,T2,T3,T4,T5,T6);
zip!(Zip7;S1,S2,S3,S4,S5,S6,S7;T1,T2,T3,T4,T5,T6,T7);
zip!(Zip8;S1,S2,S3,S4,S5,S6,S7,S8;T1,T2,T3,T4,T5,T6,T7,T8);
zip!(Zip9;S1,S2,S3,S4,S5,S6,S7,S8,S9;T1,T2,T3,T4,T5,T6,T7,T8,T9);

#[cfg(test)]
mod test {
    use futures::{executor::block_on, stream, StreamExt};

    use crate::{test_util::stuttering, Zip2, Zip3, Zip9};

    #[test]
    fn stops_at_the_shortest_of_three_sources() {
        block_on(async {
            let events = Zip3::new(
                stream::iter([1, 2, 3]),
                stream::iter([6, 7, 8, 9]),
                stream::iter([10, 11, 12]),
            )
            .collect::<Vec<_>>()
            .await;

            assert_eq!(events, [(1, 6, 10), (2, 7, 11), (3, 8, 12)]);
        });
    }

    #[test]
    fn pairs_events_by_position() {
        block_on(async {
            let events = Zip2::new(stream::iter(1..=3), stream::iter(7..=9))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 7), (2, 8), (3, 9)]);
        });
    }

    #[test]
    fn stops_at_the_shortest_source() {
        block_on(async {
            let events = Zip2::new(stream::iter(1..=5), stream::iter(7..=8))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 7), (2, 8)]);
        });
    }

    #[test]
    fn an_empty_source_yields_nothing() {
        block_on(async {
            let events = Zip2::new(stream::iter(1..=3), stream::empty::<i32>())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, []);
        });
    }

    #[test]
    fn zips_three_sources() {
        block_on(async {
            let events = Zip3::new(
                stream::iter([1, 2]),
                stream::iter(["a", "b"]),
                stream::iter([true, false]),
            )
            .collect::<Vec<_>>()
            .await;

            assert_eq!(events, [(1, "a", true), (2, "b", false)]);
        });
    }

    #[test]
    fn zips_nine_sources() {
        block_on(async {
            let events = Zip9::new(
                stream::iter([1]),
                stream::iter([2]),
                stream::iter([3]),
                stream::iter([4]),
                stream::iter([5]),
                stream::iter([6]),
                stream::iter([7]),
                stream::iter([8]),
                stream::iter([9]),
            )
            .collect::<Vec<_>>()
            .await;

            assert_eq!(events, [(1, 2, 3, 4, 5, 6, 7, 8, 9)]);
        });
    }

    #[test]
    fn survives_sources_that_are_not_always_ready() {
        block_on(async {
            let events = Zip2::new(stuttering(1..=3), stuttering(7..=9))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(events, [(1, 7), (2, 8), (3, 9)]);
        });
    }
}
