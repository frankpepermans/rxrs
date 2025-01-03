use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{Controller, Event, Observable};

use super::Subject;

type Subscription<T> = Weak<RefCell<Controller<Event<T>>>>;

pub struct PublishSubject<T> {
    subscriptions: Vec<Subscription<T>>,
    is_closed: bool,
}

impl<T> Subject for PublishSubject<T> {
    type Item = T;

    fn subscribe(&mut self) -> Observable<Self::Item> {
        let mut stream = Controller::new();

        stream.is_done = self.is_closed;

        let stream = Rc::new(RefCell::new(stream));

        self.subscriptions.push(Rc::downgrade(&stream));

        Observable::new(stream)
    }

    fn close(&mut self) {
        self.is_closed = true;

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().is_done = true;
        }
    }

    fn next(&mut self, value: Self::Item) {
        let rc = Rc::new(value);

        for sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            sub.borrow_mut().push(Event(rc.clone()));
        }
    }

    fn for_each_subscription<F: FnMut(&mut super::Subscription<Self::Item>)>(&mut self, mut f: F) {
        for mut sub in &mut self.subscriptions.iter().flat_map(|it| it.upgrade()) {
            f(&mut sub);
        }
    }
}

#[allow(clippy::new_without_default)]
impl<T> PublishSubject<T> {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            is_closed: false,
        }
    }
}

impl<T> Drop for PublishSubject<T> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod test {
    use futures::{executor::block_on, StreamExt};

    use super::*;

    #[test]
    fn subscribe_before_events() {
        let mut subject = PublishSubject::new();
        let obs = subject.subscribe();

        subject.next(1);
        subject.next(2);
        subject.next(3);
        subject.close();

        block_on(async {
            let res = obs.map(|it| *it.as_inner_ref()).collect::<Vec<i32>>().await;

            assert_eq!(res, [1, 2, 3]);
        });
    }

    #[test]
    fn subscribe_after_events() {
        let mut subject = PublishSubject::new();

        subject.next(1);
        subject.next(2);
        subject.next(3);
        subject.close();

        let obs = subject.subscribe();

        block_on(async {
            let res = obs.map(|it| *it.as_inner_ref()).collect::<Vec<i32>>().await;

            assert_eq!(res, []);
        });
    }

    #[test]
    fn ok_event_ownership() {
        let mut subject = PublishSubject::new();
        let obs = subject.subscribe();

        subject.next(1);
        subject.next(2);
        subject.next(3);
        subject.close();

        block_on(async {
            let res = obs.map(|it| it.try_unwrap()).collect::<Vec<_>>().await;

            assert_eq!(res, [Ok(1), Ok(2), Ok(3)]);
        });
    }

    #[test]
    fn err_event_ownership() {
        let mut subject = PublishSubject::new();
        let obs = subject.subscribe();
        let some_other_obs = subject.subscribe();

        subject.next(1);
        subject.next(2);
        subject.next(3);
        subject.close();

        block_on(async {
            let res = obs.map(|it| it.try_unwrap()).collect::<Vec<_>>().await;

            for it in res {
                assert!(it.is_err(), "Event was not Err()");
            }
        });

        drop(some_other_obs);
    }
}
