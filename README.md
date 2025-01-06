RxRs is a lightweight Rx implementation which build upon futures::Stream.

It aims to provide Subjects which allow multiple subscribing Streams. Events are ref-counted in the downstream(s).
The subjects are:
- PublishSubject
- BehaviorSubject
- ReplaySubject

Then there's combinators
- CombineLatest2..CombineLatest9
- Zip2..Zip9

It also exposes RxExt, which like StreamExt provides typical Rx transformers.
The ops so far are: 
- buffer
- debounce
- distinct
- distinct_until_changed
- pairwise
- race
- share
- share_behavior
- share_replay
- start_with
- switch_map
- window

*Subject example*

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

*CombineLatest example*

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

*Operators*

```rust
// ops are accessible via the RxExt trait and work on futures::Stream
let stream = stream::iter(0..=10)
    .start_with([-1, -2]) // precede the emission with event from an Iter
    .distinct_until_changed() // avoid repeating the same exact event in immediate sequence
    .buffer(|_, count| async move { count == 3 }) // buffer every 3 events emitted
    .debounce(|buffered_items| async { /* use a delay */ })
    .pairwise() // previous and next events side-by-side
    .share_behavior(); // convert into a broadcast Stream and for every new subscription, start by emitting the last emitted event
```