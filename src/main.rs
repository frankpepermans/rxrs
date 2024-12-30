use std::fmt::Display;

use futures::{executor::block_on, stream, StreamExt};
use rxrs::prelude::*;

fn main() {
    let stream_base = stream::iter(vec![_Event::new(1), _Event::new(2), _Event::new(3)]);
    let mut ctrl = BehaviorSubject::new();

    ctrl.push(1);
    ctrl.push(2);
    ctrl.push(3);
    ctrl.close();

    let stream_a = ctrl.subscribe();
    let stream_b = ctrl.subscribe();

    block_on(async {
        let res_base = stream_base
            .inspect(|it| println!("{it}"))
            .map(|it| it.value)
            .collect::<Vec<_>>()
            .await;
        let res_a = stream_a
            .inspect(|it| println!("{it}"))
            .map(|it| it.data() + 10)
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

#[derive(Debug)]
struct _Event<T> {
    value: T,
}

impl<T> _Event<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: ToString> Display for _Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value.to_string())
    }
}
