use std::rc::Rc;

#[derive(Debug)]
pub struct Event<T>(pub(crate) Rc<T>);

impl<T> Event<T> {
    pub fn as_inner_ref(&self) -> &T {
        &self.0
    }

    pub fn try_unwrap(self) -> Result<T, Rc<T>> {
        Rc::try_unwrap(self.0)
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
