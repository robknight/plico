/// The base trait for any value in a domain.
pub trait ValueEquality: Clone + std::fmt::Debug + Eq + std::hash::Hash + 'static {}
impl<T> ValueEquality for T where T: Clone + std::fmt::Debug + Eq + std::hash::Hash + 'static {}

/// A capability trait for values that can be ordered.
pub trait ValueOrdering: ValueEquality + Ord {}
impl<T> ValueOrdering for T where T: ValueEquality + Ord {}

/// A capability trait for values that support basic arithmetic.
pub trait ValueArithmetic: ValueEquality {
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
}

/// A concrete enum providing standard implementations of value capabilities.
/// Problem-specific value types can compose this to reuse standard functionality.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StandardValue {
    Int(i64),
    Bool(bool),
}

impl ValueArithmetic for StandardValue {
    fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (StandardValue::Int(a), StandardValue::Int(b)) => StandardValue::Int(a + b),
            _ => panic!("Arithmetic add is only supported for Int types"),
        }
    }

    fn sub(&self, other: &Self) -> Self {
        match (self, other) {
            (StandardValue::Int(a), StandardValue::Int(b)) => StandardValue::Int(a - b),
            _ => panic!("Arithmetic sub is only supported for Int types"),
        }
    }
}
