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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn only_next_carries_a_value() {
        assert_eq!(Notification::Next(1).inner_value(), Some(1));
        assert_eq!(Notification::<i32>::Complete.inner_value(), None);
    }

    #[test]
    fn compares_across_variants() {
        assert_eq!(Notification::Next(1), Notification::Next(1));
        assert_ne!(Notification::Next(1), Notification::Next(2));
        assert_ne!(Notification::Next(1), Notification::Complete);
        assert_eq!(Notification::<i32>::Complete, Notification::Complete);
    }

    #[test]
    fn clones_both_variants() {
        assert_eq!(Notification::Next(1).clone(), Notification::Next(1));
        assert_eq!(
            Notification::<i32>::Complete.clone(),
            Notification::Complete
        );
    }
}
