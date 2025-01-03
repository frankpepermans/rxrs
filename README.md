*Work in progress*

RxRs is a lightweight Rx implementation which build upon futures::Stream.

It aims to provide Subjects (PublishSubject, BehaviorSubject, ReplaySubject) which allow multiple subscribing Streams. Events are ref-counted in the downstream(s).

Then there's combinators, like CombineLatest and Zip.

It also exposes RxExt, which like StreamExt provides typical Rx transformers.
The ops so far are:
- race: the first Stream to emit a value is taken, the other discarded.
- share: transforms a normal Stream into a broadcast one.
- start_with: prepends the emitted events with a strting event.
- switch_map: on each event, re-subscibes to a switch-Stream and polls its values.