/// The base trait for any value that can be used in a variable's domain.
///
/// This trait establishes the minimum requirements for a value: it must be
/// cloneable, debuggable, equatable, and hashable. This is a marker trait,
/// so any type that satisfies these bounds implements `ValueEquality`.
pub trait ValueEquality: Clone + std::fmt::Debug + Eq + std::hash::Hash + 'static {}
impl<T> ValueEquality for T where T: Clone + std::fmt::Debug + Eq + std::hash::Hash + 'static {}

/// A capability trait for values that have a defined ordering.
///
/// This is used for constraints or domain representations that rely on sorting
/// or comparing values (e.g., `OrderedDomain`).
pub trait ValueOrdering: ValueEquality + Ord {}
impl<T> ValueOrdering for T where T: ValueEquality + Ord {}

/// A capability trait for values that support basic arithmetic operations.
///
/// This allows generic constraints like `SumOf` to operate on different
/// numerical types.
pub trait ValueArithmetic: ValueEquality {
    /// Adds two values.
    ///
    /// # Panics
    ///
    /// This method may panic if the underlying types do not support addition
    /// (e.g., attempting to add boolean values).
    fn add(&self, other: &Self) -> Self;
    /// Subtracts one value from another.
    ///
    /// # Panics
    ///
    /// This method may panic if the underlying types do not support subtraction.
    fn sub(&self, other: &Self) -> Self;
    /// Returns the absolute value of the value.
    ///
    /// # Panics
    ///
    /// This method may panic if the underlying type does not support the
    /// absolute value operation.
    fn abs(&self) -> Self;
}

/// A capability trait for values that can be represented as a continuous range.
///
/// This is essential for the `RangeDomain` representation, allowing it to iterate
/// and calculate its size.
pub trait ValueRange: ValueOrdering {
    /// Returns the next value in the sequence.
    ///
    /// # Panics
    ///
    /// This method may panic if the value has no logical successor (e.g., a boolean).
    fn successor(&self) -> Self;
    /// Calculates the number of steps between two values.
    ///
    /// # Panics
    ///
    /// This method may panic if a distance cannot be calculated between the types.
    fn distance(&self, other: &Self) -> u64;
}

/// A concrete enum providing standard, reusable implementations of value capabilities.
///
/// Problem-specific value types can wrap or compose `StandardValue` to easily
/// gain support for standard constraints (like arithmetic) without needing to
/// reimplement the logic.
///
/// # Example
///
/// ```no_run
/// use plico::solver::value::StandardValue;
///
/// // A custom value type for a hypothetical problem.
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// pub enum MyProblemValue {
///     DomainSpecificValue(String),
///     Standard(StandardValue),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StandardValue {
    /// A 64-bit integer value.
    Int(i64),
    /// A boolean value.
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

    fn abs(&self) -> Self {
        match self {
            StandardValue::Int(a) => StandardValue::Int(a.abs()),
            _ => panic!("Arithmetic abs is only supported for Int types"),
        }
    }
}

impl ValueRange for StandardValue {
    fn successor(&self) -> Self {
        match self {
            StandardValue::Int(a) => StandardValue::Int(a + 1),
            _ => panic!("Successor is only supported for Int types"),
        }
    }

    fn distance(&self, other: &Self) -> u64 {
        match (self, other) {
            (StandardValue::Int(a), StandardValue::Int(b)) => b.abs_diff(*a),
            _ => panic!("Distance is only supported for Int types"),
        }
    }
}
