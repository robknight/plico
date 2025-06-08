use crate::solver::{
    engine::VariableId, semantics::DomainSemantics, solution::Solution, value::ValueOrdering,
};

/// A trait for strategies that determine the order of values to try for a variable.
pub trait ValueOrderingHeuristic<S: DomainSemantics> {
    /// Given a variable's ID and the current solution, returns an iterator
    /// over the values in the order they should be tried.
    ///
    /// # Arguments
    ///
    /// * `variable_id`: The ID of the variable being branched on.
    /// * `solution`: The current partial solution.
    ///
    /// # Returns
    ///
    /// An iterator that yields the values in the desired order.
    fn order_values<'a>(
        &self,
        variable_id: VariableId,
        solution: &'a Solution<S>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a;
}

/// A simple heuristic that returns values in their natural iteration order.
///
/// **Warning:** The iteration order is not guaranteed to be the same across
/// different runs for domains that do not have a defined order (like
/// [`HashSetDomain`]), making this heuristic non-deterministic for such cases.
pub struct IdentityValueHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for IdentityValueHeuristic {
    fn order_values<'a>(
        &self,
        variable_id: VariableId,
        solution: &'a Solution<S>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        let domain = solution.domains.get(&variable_id).unwrap();
        Box::new(domain.iter().cloned())
    }
}

/// A heuristic that returns values in a stable, sorted order, ensuring
/// deterministic behavior.
///
/// This requires that the value type implements [`ValueOrdering`] (and therefore
/// `Ord`).
pub struct DeterministicIdentityValueHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for DeterministicIdentityValueHeuristic
where
    S::Value: ValueOrdering,
{
    fn order_values<'a>(
        &self,
        variable_id: VariableId,
        solution: &'a Solution<S>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        let domain = solution.domains.get(&variable_id).unwrap();
        let mut values: Vec<S::Value> = domain.iter().cloned().collect();
        values.sort();
        Box::new(values.into_iter())
    }
}

/// A heuristic that prioritizes values that have already been used by other
/// variables in the current partial solution.
///
/// This is a "resource minimization" heuristic. It steers the search towards
/// solutions that reuse existing assignments, which is useful for problems like
/// minimal graph coloring where the goal is to use as few "colors" as possible.
pub struct PreferUsedValuesHeuristic;

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for PreferUsedValuesHeuristic
where
    S::Value: Clone + Eq + std::hash::Hash + Ord,
{
    fn order_values<'a>(
        &self,
        variable_id: VariableId,
        solution: &'a Solution<S>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        let current_domain = solution.domains.get(&variable_id).unwrap();
        let domain_values: im::HashSet<S::Value> = current_domain.iter().cloned().collect();

        let used_values: im::HashSet<S::Value> = solution
            .domains
            .iter()
            .filter(|(id, _)| **id != variable_id) // Exclude the current variable
            .filter_map(|(_, domain)| domain.get_singleton_value())
            .collect();

        let preferred_values: im::HashSet<S::Value> =
            domain_values.clone().intersection(used_values.clone());
        let other_values: im::HashSet<S::Value> = domain_values.difference(used_values);

        let mut preferred_values: Vec<S::Value> = preferred_values.into_iter().collect();
        let mut other_values: Vec<S::Value> = other_values.into_iter().collect();

        // Sort the sections to ensure determinism
        preferred_values.sort();
        other_values.sort();

        Box::new(preferred_values.into_iter().chain(other_values))
    }
}

/// A meta-heuristic that dispatches to a specific heuristic based on a variable's
/// semantic metadata.
///
/// This allows for applying different value-ordering strategies to different
/// types of variables within the same solver. It is configured with a map from a
/// metadata tag to a specific heuristic, and a default heuristic to use for any
/// variable whose metadata tag is not in the map.
pub struct SwitchingValueHeuristic<S: DomainSemantics> {
    specific_heuristics:
        std::collections::HashMap<S::VariableMetadata, Box<dyn ValueOrderingHeuristic<S>>>,
    default_heuristic: Box<dyn ValueOrderingHeuristic<S>>,
}

impl<S: DomainSemantics> SwitchingValueHeuristic<S> {
    /// Creates a new `SwitchingValueHeuristic`.
    pub fn new(
        specific_heuristics: std::collections::HashMap<
            S::VariableMetadata,
            Box<dyn ValueOrderingHeuristic<S>>,
        >,
        default_heuristic: Box<dyn ValueOrderingHeuristic<S>>,
    ) -> Self {
        Self {
            specific_heuristics,
            default_heuristic,
        }
    }
}

impl<S: DomainSemantics> ValueOrderingHeuristic<S> for SwitchingValueHeuristic<S> {
    fn order_values<'a>(
        &self,
        variable_id: VariableId,
        solution: &'a Solution<S>,
    ) -> Box<dyn Iterator<Item = S::Value> + 'a>
    where
        S::Value: 'a,
    {
        let metadata = solution.variable_metadata.get(&variable_id);
        if let Some(meta) = metadata {
            if let Some(heuristic) = self.specific_heuristics.get(meta) {
                return heuristic.order_values(variable_id, solution);
            }
        }
        self.default_heuristic.order_values(variable_id, solution)
    }
}
