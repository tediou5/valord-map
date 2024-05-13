use std::sync::Arc;

pub trait OrdBy {
    type Target: Ord + Clone;
    fn ord_by(&self) -> &Self::Target;
}

impl<T: Ord + Clone> OrdBy for T {
    type Target = T;
    fn ord_by(&self) -> &Self::Target {
        self
    }
}

#[derive(Debug)]
pub struct ValueOrdBy<T: OrdBy>(pub(crate) Arc<T>);

impl<T: OrdBy> ValueOrdBy<T> {
    pub(crate) fn into_inner(self) -> Option<T> {
        Arc::<T>::into_inner(self.0)
    }

    pub(crate) fn ord_by(&self) -> &T::Target {
        self.0.ord_by()
    }
}

impl<T: OrdBy> From<T> for ValueOrdBy<T> {
    fn from(value: T) -> Self {
        ValueOrdBy(value.into())
    }
}

impl<T: OrdBy> PartialEq for ValueOrdBy<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ord_by() == other.ord_by()
    }
}

impl<T: OrdBy> Eq for ValueOrdBy<T> {}

impl<T: OrdBy> PartialOrd for ValueOrdBy<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.ord_by().cmp(other.ord_by()))
    }
}

impl<T: OrdBy> Ord for ValueOrdBy<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord_by().cmp(other.ord_by())
    }
}

impl<T: OrdBy> AsRef<T> for ValueOrdBy<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T: OrdBy> Clone for ValueOrdBy<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
