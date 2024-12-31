use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

// todo: setup this macro to allow CombineLatest[2..n]
macro_rules! combine_latest {
    ($name:ident; $($stream:ident),+; $($type:ident),+) => {
        pub struct $name<S1: Stream<Item = T1>, S2: Stream<Item = T2>, T1, T2> {
            stream_1: Pin<Box<S1>>,
            stream_2: Pin<Box<S2>>,
            latest_value_1: Option<T1>,
            latest_value_2: Option<T2>,
        }

        impl<S1: Stream<Item = T1>, S2: Stream<Item = T2>, T1, T2> $name<S1, S2, T1, T2> {
            pub fn new(stream_1: S1, stream_2: S2) -> Self {
                $name {
                    stream_1: Box::pin(stream_1),
                    stream_2: Box::pin(stream_2),
                    latest_value_1: None,
                    latest_value_2: None,
                }
            }
        }

        impl<
                S1: Stream<Item = T1>,
                S2: Stream<Item = T2>,
                T1: Clone + Unpin,
                T2: Clone + Unpin,
            > Stream for $name<S1, S2, T1, T2>
        {
            type Item = (T1, T2);

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

                let (event_1, is_done_1) = poll_next(&mut this.stream_1, cx);
                let (event_2, is_done_2) = poll_next(&mut this.stream_2, cx);
                let did_update_value = event_1.is_some() || event_2.is_some();

                if event_1.is_some() {
                    this.latest_value_1 = event_1;
                }

                if event_2.is_some() {
                    this.latest_value_2 = event_2;
                }

                let all_done = is_done_1 && is_done_2;
                let all_emitted = this.latest_value_1.is_some() && this.latest_value_2.is_some();
                let should_emit_next = all_emitted && did_update_value;

                match (all_done, should_emit_next) {
                    (true, _) => Poll::Ready(None),
                    (false, true) => Poll::Ready(Some((
                        this.latest_value_1.as_ref().unwrap().to_owned(),
                        this.latest_value_2.as_ref().unwrap().to_owned(),
                    ))),
                    _ => Poll::Pending,
                }
            }
        }
    };
}

combine_latest!(CombineLatest2; S1, S2; T1, T2);

#[test]
fn test() {
    use futures::executor::block_on;
    use futures::stream::{self};
    use futures::StreamExt;

    let s1 = stream::iter([1, 2, 3]);
    let s2 = stream::iter([6, 7, 8, 9]);
    let mut stream = CombineLatest2::new(s1, s2);

    block_on(async {
        while let Some(it) = stream.next().await {
            println!("{:?}", it);
        }
    });
}
