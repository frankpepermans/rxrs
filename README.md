# futures-rx: lightweight Rx implementation built upon `futures::Stream`
[![](https://docs.rs/futures-rx/badge.svg)](https://docs.rs/futures-rx/latest/futures_rx/stream_ext/trait.RxExt.html)
[![](https://img.shields.io/crates/v/futures-rx.svg)](https://crates.io/crates/futures-rx)
[![](https://img.shields.io/crates/d/futures-rx.svg)](https://crates.io/crates/futures-rx)

## Subjects

Subjects are `Stream` controllers, that allow pushing new events to them, comparable to collections.
You can subscribe to them, which returns an `Observable`, which just implements `Stream`.

This `Observable` can be polled, but all items are wrapped in an `Event` struct,
which internally handles an `Rc` containing a reference to the actual item.

The subjects are:
- `PublishSubject`
- `BehaviorSubject`
- `ReplaySubject`

Subjects are hot observables, meaning you can subscribe to them as much as you like and at any point in time,
but you will miss out on items that have been polled _before_ subscribing.

`PublishSubject` is the default version, acting as explained above.
However, a `BehaviorSubject` will always replay the last emitted item to any new subscription
and `ReplaySubject` will replay _all_ events from the beginning. `ReplaySubject` can also take a buffer size, to avoid memory issues when dealing with massive amounts of events.

```rust
let mut subject = BehaviorSubject::new();

subject.next(1);
subject.next(2);
subject.next(3);
subject.close();

let obs = subject.subscribe();
// You can subscribe multiple times
let another_obs = subject.subscribe();

block_on(async {
    // Since Subjects allow for multiple subscribers, events are
    // wrapped in Event types, which internally manage an Rc to the actual event.
    // Here, we just borrow the underlying value and deref it.
    let res = obs.map(|it| *it.borrow_value()).collect::<Vec<i32>>().await;

    assert_eq!(res, [3]);
});
```

## Combine

Currently there's 2 macro-generated `Stream` builders:
- `CombineLatest2`..`CombineLatest9`
- `Zip2`..`Zip9`

`CombineLatest` emits all latest items from n-`Stream`s

```rust
let s1 = stream::iter([1, 2, 3]);
let s2 = stream::iter([6, 7, 8, 9]);
let s3 = stream::iter([0]);
let stream = CombineLatest3::new(s1, s2, s3);

block_on(async {
    let res = stream.collect::<Vec<_>>().await;

    assert_eq!(res, [(1, 6, 0), (2, 7, 0), (3, 8, 0), (3, 9, 0),]);
});
```

`Zip` is similar, but instead emits all combined items by sequence:

```rust
let s1 = stream::iter([1, 2, 3]);
let s2 = stream::iter([6, 7, 8, 9]);
let stream = Zip2::new(s1, s2);

block_on(async {
    let res = stream.collect::<Vec<_>>().await;

    assert_eq!(res, [(1, 6), (2, 7), (3, 8),]);
});
```

## Ops

futures-rx also exposes the `RxExt` trait, which, like `StreamExt`, provides typical Rx transformers.

Note that a lot of other Rx operators are already part of the `futures::StreamExt` trait. This crate will only ever contain Rx operators that are missing from `StreamExt`.
Do use both `StreamExt` and `RxExt` to access all.

- `buffer`
```rust
futures::executor::block_on(async {
    use futures::stream::{self, StreamExt};
    use futures_rx::RxExt;

    let stream = stream::iter(0..9);
    let stream = stream.window(|_, count| async move { count == 3 }).flat_map(|it| it);

    assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8], stream.collect::<Vec<_>>().await);
});
```

- `debounce`
```rust
futures::executor::block_on(async {
    stream
        .debounce(|_| Duration::from_millis(150).into_future())
        .collect::<Vec<_>>()
        .await;
});
```

- `delay`
```rust
futures::executor::block_on(async {
    let now = SystemTime::now();
    let all_events = stream::iter(0..=3)
        .delay(|| Duration::from_millis(100).into_future())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 1, 2, 3]);
    assert!(now.elapsed().unwrap().as_millis() >= 100);
});    
```

- `delay_every`
```rust
futures::executor::block_on(async {
    let now = Instant::now();
    let all_events = stream::iter(0..=3)
        .delay_every(|_| Duration::from_millis(50).into_future(), None)
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 1, 2, 3]);
    assert!(now.elapsed().as_millis() >= 50 * 4);
});    
```

- `dematerialize`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(1..=2);
    let all_events = stream
        .materialize()
        .dematerialize()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [1, 2]);
});    
```

- `distinct`
```rust
futures::executor::block_on(async {
    let stream = stream::iter([1, 1, 2, 1, 3, 2, 4, 5]);
    let all_events = stream.distinct().collect::<Vec<_>>().await;

    assert_eq!(all_events, [1, 2, 3, 4, 5]);
});    
```

- `distinct_until_changed`
```rust
futures::executor::block_on(async {
    let stream = stream::iter([1, 1, 2, 3, 3, 3, 4, 5]);
    let all_events = stream.distinct_until_changed().collect::<Vec<_>>().await;

    assert_eq!(all_events, [1, 2, 3, 4, 5]);
});    
```

- `end_with`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(1..=5);
    let all_events = stream.end_with([0]).collect::<Vec<_>>().await;

    assert_eq!(all_events, [1, 2, 3, 4, 5, 0]);
});    
```

- `inspect_done`
```rust
futures::executor::block_on(async {
    let mut is_done = false;
    let all_events = stream::iter(0..=3)
        .inspect_done(|| is_done = true)
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 1, 2, 3]);
    assert!(is_done);
});    
```

- `materialize`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(1..=2);
    let all_events = stream.materialize().collect::<Vec<_>>().await;

    assert_eq!(
        all_events,
        [
            Notification::Next(1),
            Notification::Next(2),
            Notification::Complete
        ]
    );
});    
```

- `pairwise`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(0..=5);
    let all_events = stream
        .pairwise()
        .map(|(prev, next)| (prev, *next))
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
});    
```

- `race`
```rust
futures::executor::block_on(async {
    let mut phase = 0usize;
    let fast_stream = stream::iter(["fast"]);
    let slow_stream = stream::poll_fn(move |_| {
        // let's make it slower by first emitting a Pending state
        phase += 1;

        match phase {
            1 => Poll::Pending,
            2 => Poll::Ready(Some("slow")),
            3 => Poll::Ready(None),
            _ => unreachable!(),
        }
    });
    let all_events = slow_stream.race(fast_stream).collect::<Vec<_>>().await;

    assert_eq!(all_events, ["fast"]);
});    
```

- `sample`
```rust
futures::executor::block_on(async {
    let stream = create_stream(); // produces over time, interval is 20ms
        .take(6)
        .enumerate()
        .map(|(index, _)| index);
    let sampler = futures_time::stream::interval(Duration::from_millis(50)).take(6);
    let all_events = stream.sample(sampler).collect::<Vec<_>>().await;

    assert_eq!(all_events, [1, 3, 5]);
});    
```

- `share`
- `share_behavior`
- `share_replay`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(1usize..=3usize);
    let s1 = stream.share(); // first subscription
    let s2 = s1.clone(); // second subscription
    let (a, b) = join(s1.collect::<Vec<_>>(), s2.collect::<Vec<_>>()).await;

    // as s1 and s2 produce Events, which wrap an Rc
    // we can call into() on the test values to convert them into Events as well.
    assert_eq!(a, [1.into(), 2.into(), 3.into()]);
    assert_eq!(b, [1.into(), 2.into(), 3.into()]);
});    
```

- `start_with`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(1..=5);
    let all_events = stream.start_with([0]).collect::<Vec<_>>().await;

    assert_eq!(all_events, [0, 1, 2, 3, 4, 5]);
});    
```

- `switch_map`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(0usize..=3usize);
    let all_events = stream
        .switch_map(|i| stream::iter([i.pow(2), i.pow(3), i.pow(4)]))
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 1, 4, 9, 27, 81]);
});    
```

- `throttle`
```rust
futures::executor::block_on(async {
    let stream = create_stream(); // produces 0..=9 over time, interval is 50ms
    let all_events = stream
        .throttle(|_| Duration::from_millis(175).into_future())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 4, 8]);
});    
```

- `throttle_trailing`
```rust
futures::executor::block_on(async {
    let stream = create_stream(); // produces 0..=9 over time, interval is 50ms
    let all_events = stream
        .throttle_trailing(|_| Duration::from_millis(175).into_future())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [3, 7]);
});    
```

- `throttle_all`
```rust
futures::executor::block_on(async {
    let stream = create_stream(); // produces 0..=9 over time, interval is 50ms
    let all_events = stream
        .throttle_all(|_| Duration::from_millis(175).into_future())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(all_events, [0, 3, 4, 7, 8]);
});    
```

- `timing`
```rust
futures::executor::block_on(async {
    let stream = create_stream(); // produces 0..=9 over time, interval is 50ms
    let start = Instant::now();
    let all_events = stream.timing().collect::<Vec<_>>().await;
    let timestamps = all_events
        .iter()
        .map(|it| it.timestamp)
        .enumerate()
        .collect::<Vec<_>>();
    let intervals = all_events
        .iter()
        .map(|it| it.interval)
        .enumerate()
        .collect::<Vec<_>>();

    for (index, timestamp) in timestamps {
        assert!(
            timestamp.duration_since(start).as_millis() >= (50 * index).try_into().unwrap()
        );
    }

    for (index, interval) in intervals {
        if index == 0 {
            assert!(interval.is_none());
        } else {
            assert!(interval.expect("interval is None!").as_millis() >= 50);
        }
    }
});    
```

- `window`
```rust
futures::executor::block_on(async {
    let all_events = stream::iter(0..=8)
        .window(|_, count| async move { count == 3 })
        .enumerate()
        .flat_map(|(index, it)| it.map(move |it| (index, it)))
        .collect::<Vec<_>>()
        .await;

    assert_eq!(
        all_events,
        vec![
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (2, 6),
            (2, 7),
            (2, 8)
        ]
    );
});    
```

- `with_latest_from`
```rust
futures::executor::block_on(async {
    let stream = stream::iter(0..=3);
    let stream = stream.with_latest_from(stream::iter(0..=3));

    assert_eq!(vec![(0, 0), (1, 1), (2, 2), (3, 3)], stream.collect::<Vec<_>>().await);
});    
```