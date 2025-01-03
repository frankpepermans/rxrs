use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use paste::paste;
use pin_project_lite::pin_project;
use std::cmp::Ordering;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! combine_latest {
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

        impl<$($stream: Stream<Item = $type>),+, $($type: ToOwned<Owned = $type>),+> FusedStream for $name<$($stream),+, $($type),+>
        {
            fn is_terminated(&self) -> bool {
                paste! {
                    $(
                        self.[<$stream:lower>].is_terminated()
                    )&&+
                }
            }
        }

        impl<$($stream: Stream<Item = $type>),+, $($type: ToOwned<Owned = $type>),+> Stream for $name<$($stream),+, $($type),+>
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
                    let mut did_update_value = false;
                    $(
                        if !this.[<$stream:lower>].is_terminated() {
                            let next = poll_next(this.[<$stream:lower>].as_mut(), cx);

                            if !did_update_value {
                                did_update_value = next.is_some();
                            }

                            if next.is_some() {
                                *this.[<$type:lower>] = next;
                            }
                        };
                    )+

                    if did_update_value && $(this.[<$type:lower>].is_some())&&+ {
                        // maybe to_owned can be avoided? Event/Rc?
                        Poll::Ready(Some((
                            $(
                                this.[<$type:lower>].as_ref().unwrap().to_owned()
                            ),+
                        )))
                    } else if $(this.[<$stream:lower>].is_terminated())&&+ {
                        Poll::Ready(None)
                    } else {
                        Poll::Pending
                    }
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                paste! {
                    let size_hint_all = [$(self.[<$stream:lower>].size_hint()),+];
                    let upper = if $(self.[<$stream:lower>].is_terminated())&&+ {
                        size_hint_all
                            .into_iter()
                            .max_by(|a, b| match (a.1, b.1) {
                                (None, None) => Ordering::Equal,
                                (None, Some(_)) => Ordering::Greater,
                                (Some(_), None) => Ordering::Less,
                                (Some(a), Some(b)) => a.cmp(&b),
                        }).unwrap().1
                    } else {
                        None
                    };
                }

                (
                    size_hint_all
                        .iter()
                        .max_by(|a, b| a.0.cmp(&b.0))
                        .unwrap().0,
                    upper
                )
            }
        }
    };
}

combine_latest!(CombineLatest2;S1,S2;T1,T2);
combine_latest!(CombineLatest3;S1,S2,S3;T1,T2,T3);
combine_latest!(CombineLatest4;S1,S2,S3,S4;T1,T2,T3,T4);
combine_latest!(CombineLatest5;S1,S2,S3,S4,S5;T1,T2,T3,T4,T5);
combine_latest!(CombineLatest6;S1,S2,S3,S4,S5,S6;T1,T2,T3,T4,T5,T6);
combine_latest!(CombineLatest7;S1,S2,S3,S4,S5,S6,S7;T1,T2,T3,T4,T5,T6,T7);
combine_latest!(CombineLatest8;S1,S2,S3,S4,S5,S6,S7,S8;T1,T2,T3,T4,T5,T6,T7,T8);
combine_latest!(CombineLatest9;S1,S2,S3,S4,S5,S6,S7,S8,S9;T1,T2,T3,T4,T5,T6,T7,T8,T9);

#[test]
fn test() {
    use futures::executor::block_on;
    use futures::stream::{self};
    use futures::StreamExt;

    let s1 = stream::iter([1, 2, 3]);
    let s2 = stream::iter([6, 7, 8, 9]);
    let s3 = stream::iter([0]);
    let stream = CombineLatest3::new(s1, s2, s3);

    block_on(async {
        let res = stream.collect::<Vec<_>>().await;

        assert_eq!(res, [(1, 6, 0), (2, 7, 0), (3, 8, 0), (3, 9, 0),]);
    });
}
