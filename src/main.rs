use std::fmt::Display;

use futures::{executor::block_on, StreamExt};
use rxrs::prelude::*;

fn main() {
    let mut ctrl = PublishSubject::new();
    let stream_a = ctrl.subscribe();
    let stream_b = ctrl.subscribe();

    ctrl.push(_Event::new(1));
    ctrl.push(_Event::new(2));
    ctrl.push(_Event::new(3));
    ctrl.close();

    drop(stream_a);

    //let stream_a = ctrl.subscribe();

    block_on(async {
        let res_b = stream_b
            .inspect(|it| println!("{:?}", it))
            .map(|it| it.try_unwrap())
            .collect::<Vec<_>>()
            .await;

        println!("{:?}", res_b);
    });
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
