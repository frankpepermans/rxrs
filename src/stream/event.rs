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
