use std::fmt::Display;

use futures::{executor::block_on, stream, StreamExt};
use rxrs::prelude::*;

fn main() {
    let stream_base = stream::iter(vec![Event::new(1), Event::new(2), Event::new(3)]);
    let mut ctrl = BehaviorSubject::new();

    ctrl.push(Event::new(1));
    ctrl.push(Event::new(2));
    ctrl.push(Event::new(3));
    ctrl.close();

    let stream_a = ctrl.subscribe();
    let stream_b = ctrl.subscribe();

    block_on(async {
        let res_base = stream_base
            .inspect(|it| println!("{it}"))
            .collect::<Vec<_>>()
            .await;
        let res_a = stream_a
            .inspect(|it| println!("{it}"))
            .collect::<Vec<_>>()
            .await;
        let res_b = stream_b
            .inspect(|it| println!("{it}"))
            .collect::<Vec<_>>()
            .await;

        println!("{:?}", res_base);
        println!("{:?}", res_a);
        println!("{:?}", res_b);
    });

    ctrl.close();
}

#[derive(Debug, Clone)]
struct Event<T> {
    value: T,
}

impl<T> Event<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: ToString> Display for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value.to_string())
    }
}
