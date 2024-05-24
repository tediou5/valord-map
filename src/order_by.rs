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
