#[derive(Debug)]
pub enum Notification<T> {
    Next(T),
    Complete,
}

impl<T> Notification<T> {
    pub fn inner_value(self) -> Option<T> {
        match self {
            Notification::Next(it) => Some(it),
            Notification::Complete => None,
        }
    }
}

impl<T: PartialEq> PartialEq for Notification<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Next(l0), Self::Next(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl<T: Clone> Clone for Notification<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Next(arg0) => Self::Next(arg0.clone()),
            Self::Complete => Self::Complete,
        }
    }
}
