use std::{
    fmt::{Display, Formatter, Result},
    rc::Rc,
};

#[derive(Debug)]
pub struct Event<T>(pub(crate) Rc<T>);

impl<T> Event<T> {
    pub fn data(&self) -> &T {
        &self.0
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: ToString> Display for Event<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.data().to_string())
    }
}
