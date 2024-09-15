pub trait Price {
    fn price(&self) -> f64;
}

pub trait Fulfillable<T> {
    fn fulfills(&self, other: &T) -> bool;
}
