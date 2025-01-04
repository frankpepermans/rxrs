use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime},
};

use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`debounce`](RxStreamExt::debounce) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Debounce<S: Stream> {
        #[pin]
        stream: Fuse<S>,
        duration: Duration,
        last_emission: Option<SystemTime>,
        last_event: Option<S::Item>,
    }
}

impl<S: Stream> Debounce<S> {
    pub(crate) fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream: stream.fuse(),
            duration,
            last_emission: None,
            last_event: None,
        }
    }
}

impl<S> FusedStream for Debounce<S>
where
    S: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<S> Stream for Debounce<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        while let Poll::Ready(item) = this.stream.as_mut().poll_next(cx) {
            match item {
                Some(event) => {
                    let mut next = None;
                    let now = SystemTime::now();

                    if let Some(last) = this.last_emission.replace(now) {
                        if now.duration_since(last).unwrap() >= *this.duration {
                            next = this.last_event.take();
                        }
                    }

                    this.last_event.replace(event);

                    if let Some(next) = next {
                        return Poll::Ready(Some(next));
                    }
                }
                None => return Poll::Ready(this.last_event.take()),
            }
        }

        Poll::Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.stream.size_hint();

        (a + 1, b.map(|it| it + 1))
    }
}

#[cfg(test)]
mod test {
    use std::{thread, time::Duration};

    use futures::{executor::block_on, stream, Stream, StreamExt};

    use crate::RxExt;

    #[test]
    fn smoke() {
        let stream = create_stream();

        block_on(async {
            let all_events = stream
                .debounce(Duration::from_millis(100))
                .collect::<Vec<_>>()
                .await;

            assert_eq!(all_events, [2, 6, 9]);
        });
    }

    fn create_stream() -> impl Stream<Item = usize> {
        stream::unfold(0, move |count| async move {
            if count < 10 {
                let interval = match count {
                    3 | 7 => Duration::from_millis(150),
                    _ => Duration::from_millis(50),
                };

                thread::sleep(interval);

                Some((count, count + 1))
            } else {
                None
            }
        })
    }
}
