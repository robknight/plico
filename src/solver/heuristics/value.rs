use crate::solver::{semantics::DomainSemantics, solution::Domain};

/// A trait for strategies that determine the order of values to try for a variable.
pub trait ValueOrderingHeuristic<S: DomainSemantics> {
    /// Given a variable's domain, returns an iterator over the values in the
    /// order they should be tried.
    ///
    /// # Arguments
    ///
    /// * `domain`: The domain of the variable being branched on.
    ///
    /// # Returns
    ///
    /// An iterator that yields references to the values in the desired order.
    fn order_values<'a>(
        &self,
        domain: &'a Domain<S::Value>,
    ) -> Box<dyn Iterator<Item = &'a S::Value> + 'a>;
}

/// A simple heuristic that returns values in their natural iteration order.
pub struct IdentityValueHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for IdentityValueHeuristic {
    fn order_values<'a>(
        &self,
        domain: &'a Domain<S::Value>,
    ) -> Box<dyn Iterator<Item = &'a S::Value> + 'a> {
        domain.iter()
    }
}
