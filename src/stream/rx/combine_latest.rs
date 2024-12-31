use futures::stream::Stream;
use paste::paste;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! combine_latest {
    ($name:ident; $($stream:ident),+; $($type:ident),+) => {
        paste! {
            pub struct $name<$($stream: Stream<Item = $type>),+, $($type),+> {
                $(
                    [<$stream:lower>]: Pin<Box<$stream>>,
                    [<$type:lower>]: Option<$type>,
                )+
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
                            [<$stream:lower>]: Box::pin([<$stream:lower>]),
                            [<$type:lower>]: None,
                        )+
                    }
                }
            }
        }

        impl<$($stream: Stream<Item = $type>),+, $($type: Clone + Unpin),+> Stream for $name<$($stream),+, $($type),+>
        {
            type Item = ($($type),+);

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let this = self.get_mut();

                fn poll_next<S: Stream<Item = T>, T>(
                    stream: &mut Pin<Box<S>>,
                    cx: &mut Context<'_>,
                ) -> (Option<T>, bool) {
                    match stream.as_mut().poll_next(cx) {
                        Poll::Ready(Some(it)) => (Some(it), false),
                        Poll::Ready(None) => (None, true),
                        Poll::Pending => (None, false),
                    }
                }

                paste! {
                    $(
                        let ([<event_ $stream:lower>], [<is_done_ $stream:lower>]) = poll_next(&mut this.[<$stream:lower>], cx);
                    )+
                    let did_update_value = $(
                        [<event_ $stream:lower>].is_some()
                    )||+;
                    $(
                        if [<event_ $stream:lower>].is_some() {
                            this.[<$type:lower>] = [<event_ $stream:lower>];
                        }
                    )+
                    let all_done = $(
                        [<is_done_ $stream:lower>]
                    )&&+;
                    let all_emitted = $(
                        this.[<$type:lower>].is_some()
                    )&&+;

                    let should_emit_next = all_emitted && did_update_value;

                    match (all_done, should_emit_next) {
                        (true, _) => Poll::Ready(None),
                        (false, true) => Poll::Ready(Some((
                            $(
                                this.[<$type:lower>].as_ref().unwrap().to_owned()
                            ),+
                        ))),
                        _ => Poll::Pending,
                    }
                }
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
