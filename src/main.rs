use futures::{executor::block_on, StreamExt};
use rxrs::{BehaviorSubject, PublishSubject, Subject};

fn main() {
    let mut ctrl = BehaviorSubject::new();

    ctrl.push(1);
    ctrl.push(2);
    ctrl.push(3);
    ctrl.close();

    let stream_a = ctrl.subscribe();
    let stream_b = ctrl.subscribe();

    drop(stream_a);

    //let stream_a = stream_a.as_stream().clone();
    let stream_b = stream_b.as_stream();

    block_on(async {
        /*let res_a = stream_a
        .inspect(|it| println!("{it}"))
        .collect::<Vec<_>>()
        .await;*/
        let res_b = stream_b
            .inspect(|it| println!("{it}"))
            .collect::<Vec<_>>()
            .await;

        //println!("{:?}", res_a);
        println!("{:?}", res_b);
    });

    ctrl.close();
}
