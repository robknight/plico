use crate::solver::{semantics::DomainSemantics, solution::Domain, value::ValueOrdering};

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
    /// An iterator that yields the values in the desired order.
    fn order_values<'a>(
        &self,
        domain: &'a Domain<S::Value>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a;
}

/// A simple heuristic that returns values in their natural iteration order.
/// This is NOT deterministic for domains like HashSetDomain.
pub struct IdentityValueHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for IdentityValueHeuristic {
    fn order_values<'a>(
        &self,
        domain: &'a Domain<S::Value>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        Box::new(domain.iter().cloned())
    }
}

/// A heuristic that returns values in a stable, sorted order.
/// This requires that the values can be ordered.
pub struct DeterministicIdentityValueHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for DeterministicIdentityValueHeuristic
where
    S::Value: ValueOrdering,
{
    fn order_values<'a>(
        &self,
        domain: &'a Domain<S::Value>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        let mut values: Vec<S::Value> = domain.iter().cloned().collect();
        values.sort();
        Box::new(values.into_iter())
    }
}
