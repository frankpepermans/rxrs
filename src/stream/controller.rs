use std::{
    collections::VecDeque,
    task::{Context, Poll, Waker},
};

#[derive(Clone)]
pub struct Controller<T> {
    buffer: VecDeque<T>,
    /// Only ever assigned directly when seeding a fresh `Controller` for a
    /// subscription that joins an already closed `Subject` - at that point
    /// there is no subscriber to wake yet. Every later transition to done
    /// goes through [`Controller::close`].
    pub(crate) is_done: bool,
    /// The waker of the single `Observable` reading from this controller, if
    /// it is currently parked on an empty buffer.
    waker: Option<Waker>,
}

impl<T> Controller<T> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            is_done: false,
            waker: None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn push(&mut self, value: T) {
        self.buffer.push_back(value);

        self.wake();
    }

    pub(crate) fn close(&mut self) {
        self.is_done = true;

        self.wake();
    }

    pub(crate) fn pop(&mut self, cx: &Context<'_>) -> Poll<Option<T>> {
        match self.buffer.pop_front() {
            Some(it) => Poll::Ready(Some(it)),
            None => {
                if self.is_done {
                    Poll::Ready(None)
                } else {
                    self.register(cx.waker());

                    Poll::Pending
                }
            }
        }
    }

    fn register(&mut self, waker: &Waker) {
        match &self.waker {
            Some(it) if it.will_wake(waker) => {}
            _ => self.waker = Some(waker.clone()),
        }
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::task::{Wake, Waker};

    use super::*;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn counting_waker() -> (Arc<CountingWaker>, Waker) {
        let inner = Arc::new(CountingWaker(AtomicUsize::new(0)));

        (Arc::clone(&inner), Waker::from(inner))
    }

    #[test]
    fn pops_in_order() {
        let (_, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::new();

        controller.push(1);
        controller.push(2);

        assert_eq!(controller.len(), 2);
        assert_eq!(controller.pop(&cx), Poll::Ready(Some(1)));
        assert_eq!(controller.pop(&cx), Poll::Ready(Some(2)));
        assert_eq!(controller.pop(&cx), Poll::Pending);
    }

    #[test]
    fn an_empty_closed_controller_is_done() {
        let (_, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::<i32>::new();

        controller.close();

        assert_eq!(controller.pop(&cx), Poll::Ready(None));
    }

    #[test]
    fn a_closed_controller_drains_before_completing() {
        let (_, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::new();

        controller.push(1);
        controller.close();

        assert_eq!(controller.pop(&cx), Poll::Ready(Some(1)));
        assert_eq!(controller.pop(&cx), Poll::Ready(None));
    }

    #[test]
    fn a_push_wakes_a_parked_reader() {
        let (counter, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::new();

        assert_eq!(controller.pop(&cx), Poll::Pending);
        assert_eq!(counter.0.load(Ordering::Relaxed), 0);

        controller.push(1);

        assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_close_wakes_a_parked_reader() {
        let (counter, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::<i32>::new();

        assert_eq!(controller.pop(&cx), Poll::Pending);

        controller.close();

        assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_push_without_a_parked_reader_wakes_nobody() {
        let (counter, waker) = counting_waker();
        let cx = Context::from_waker(&waker);
        let mut controller = Controller::new();

        controller.push(1);

        assert_eq!(counter.0.load(Ordering::Relaxed), 0);
        assert_eq!(controller.pop(&cx), Poll::Ready(Some(1)));
        assert_eq!(counter.0.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn only_the_most_recent_waker_is_woken() {
        let (first, first_waker) = counting_waker();
        let (second, second_waker) = counting_waker();
        let mut controller = Controller::new();

        assert_eq!(
            controller.pop(&Context::from_waker(&first_waker)),
            Poll::Pending
        );
        assert_eq!(
            controller.pop(&Context::from_waker(&second_waker)),
            Poll::Pending
        );

        controller.push(1);

        assert_eq!(first.0.load(Ordering::Relaxed), 0);
        assert_eq!(second.0.load(Ordering::Relaxed), 1);
    }
}
