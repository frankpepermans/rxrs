use std::{ops::Deref, sync::Arc};

#[derive(Debug)]
pub struct Event<T>(pub(crate) Arc<T>);

impl<T> Event<T> {
    pub fn borrow_value(&self) -> &T {
        &self.0
    }

    pub fn try_unwrap(self) -> Result<T, Arc<T>> {
        Arc::try_unwrap(self.0)
    }
}

impl<T: Clone> Event<T> {
    pub fn unwrap(self) -> T {
        Arc::unwrap_or_clone(self.0)
    }
}

impl<T> Deref for Event<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.borrow_value()
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: PartialEq> PartialEq for Event<T> {
    fn eq(&self, other: &Self) -> bool {
        self.borrow_value() == other.borrow_value()
    }
}

impl<T> From<T> for Event<T> {
    fn from(value: T) -> Self {
        Event(value.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn derefs_to_the_inner_value() {
        let event = Event::from(String::from("value"));

        assert_eq!(event.borrow_value(), "value");
        assert_eq!(event.len(), 5);
        assert_eq!(&*event, "value");
    }

    #[test]
    fn clones_share_one_value() {
        let event = Event::from(vec![1, 2, 3]);
        let clone = event.clone();

        assert!(std::ptr::eq(event.borrow_value(), clone.borrow_value()));
    }

    #[test]
    fn try_unwrap_yields_ownership_of_a_sole_reference() {
        assert_eq!(Event::from(1).try_unwrap(), Ok(1));
    }

    #[test]
    fn try_unwrap_fails_while_a_reference_is_shared() {
        let event = Event::from(1);
        let clone = event.clone();

        assert!(event.try_unwrap().is_err());

        drop(clone);
    }

    #[test]
    fn unwrap_clones_a_shared_value() {
        let event = Event::from(1);
        let clone = event.clone();

        assert_eq!(event.unwrap(), 1);
        assert_eq!(clone.unwrap(), 1);
    }

    #[test]
    fn compares_by_value_and_not_by_reference() {
        assert_eq!(Event::from(1), Event::from(1));
        assert_ne!(Event::from(1), Event::from(2));
    }
}
