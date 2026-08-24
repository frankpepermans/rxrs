//! Search-as-you-type, spread over four threads.
//!
//! A typist thread pushes keystrokes into a `PublishSubject`, which fans them out to a
//! searcher and an auditor. The searcher debounces those keystrokes into queries, runs a
//! search per query, and multicasts the hits to two consumers with `share`.
//!
//! Run it with `cargo run --example typeahead`.

use std::thread;
use std::time::Duration as StdDuration;

use futures::{executor::block_on, future::join, stream, Stream, StreamExt};
use futures_rx::{PublishSubject, ReplaySubject, RxExt, Subject};
use futures_time::{future::IntoFuture, time::Duration};

/// How long typing must pause before the text so far counts as a query.
const DEBOUNCE_WINDOW: u64 = 120;
/// How long a search takes to produce each of its results.
const RESULT_INTERVAL: u64 = 60;
const BATCH_SIZE: usize = 4;

/// Simulated typing: the text so far, and the pause that follows it.
/// Only the pauses longer than `DEBOUNCE_WINDOW` become queries.
const KEYSTROKES: [(&str, u64); 14] = [
    ("r", 40),
    ("ru", 40),
    ("rus", 40),
    ("rust", 260),
    ("rus", 40),
    ("rust", 260),
    ("rust s", 40),
    ("rust st", 40),
    ("rust str", 150),
    ("rust stre", 40),
    ("rust strea", 40),
    ("rust stream", 150),
    ("rust streams", 40),
    ("rust streams!", 400),
];

/// Stands in for a real search: three results trickling in over time, so a query
/// arriving mid-flight can visibly cut the previous one short.
fn search(query: String) -> impl Stream<Item = String> {
    stream::iter(1..=3)
        .map(move |rank| format!("{query} -> hit {rank}"))
        .delay_every(
            |_| Duration::from_millis(RESULT_INTERVAL).into_future(),
            None,
        )
}

fn main() {
    // A subject is a hand-driven source. Both subscriptions are taken up front,
    // since the typist thread takes ownership of the subject itself.
    let mut keystrokes = PublishSubject::new();
    let typed = keystrokes.subscribe();
    let audited = keystrokes.subscribe();

    let typist = thread::spawn(move || {
        for (keystroke, pause) in KEYSTROKES {
            println!("[typist]   {keystroke:?}");

            keystrokes.next(keystroke.to_string());

            thread::sleep(StdDuration::from_millis(pause));
        }

        // Completes every subscription, which is what lets the other threads
        // finish rather than parking forever.
        keystrokes.close();
    });

    let searcher = thread::spawn(move || {
        let mut results = ReplaySubject::new();

        let hits = typed
            // subscribers receive an `Event`, a handle to a ref counted value
            .map(|it| it.unwrap())
            // collapse a burst of typing down to the text left at the pause
            .debounce(|_| Duration::from_millis(DEBOUNCE_WINDOW).into_future())
            // backspacing and retyping the same text is not a new query
            .distinct_until_changed()
            // a new query abandons the search still in flight
            .switch_map(search)
            // two consumers below want these hits; without `share` the whole
            // pipeline above would run once per consumer, searches included
            .share();

        let rendered = hits.clone();

        block_on(async {
            // both clones have to be polled for the shared source to advance
            join(
                rendered.for_each(|hit| async move { println!("[render]   {}", *hit) }),
                async {
                    let mut recorded = Box::pin(hits);

                    while let Some(hit) = recorded.next().await {
                        println!("[record]   {}", *hit);

                        results.next(hit.unwrap());
                    }
                },
            )
            .await;
        });

        // a `ReplaySubject` hands its whole history to whoever subscribes, so this
        // subscription still sees every hit that went past before it existed
        let replayed = results.subscribe();

        results.close();

        block_on(async {
            println!(
                "[record]   a late subscriber replayed {} hits",
                replayed.count().await
            );
        });
    });

    let auditor = thread::spawn(move || {
        block_on(async {
            // the same keystrokes the searcher sees, grouped into fixed batches
            let mut batches =
                Box::pin(audited.buffer(|_, count| async move { count == BATCH_SIZE }));

            while let Some(batch) = batches.next().await {
                println!("[auditor]  batch of {} keystrokes", batch.len());
            }
        });
    });

    for handle in [typist, searcher, auditor] {
        handle.join().unwrap();
    }
}
