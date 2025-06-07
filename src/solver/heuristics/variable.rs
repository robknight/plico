use crate::solver::{engine::VariableId, semantics::DomainSemantics, solution::Solution};

/// A trait for strategies that select the next variable to branch on during search.
pub trait VariableSelectionHeuristic<S: DomainSemantics> {
    /// Selects a variable from the current solution state.
    ///
    /// # Arguments
    ///
    /// * `solution`: The current partial solution, containing the domains of all variables.
    ///
    /// # Returns
    ///
    /// * `Some(VariableId)` of an unassigned variable, if any exist.
    /// * `None` if all variables are already assigned (i.e., their domains are singletons).
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId>;
}

/// A simple heuristic that selects the first unassigned variable it finds.
pub struct SelectFirstHeuristic;

impl<S: DomainSemantics> VariableSelectionHeuristic<S> for SelectFirstHeuristic {
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId> {
        solution
            .domains
            .iter()
            .find(|(_, domain)| domain.len() > 1)
            .map(|(var_id, _)| *var_id)
    }
}

/// A heuristic that selects the variable with the Minimum Remaining Values (MRV) in its domain.
/// This is a "fail-first" strategy, aiming to tackle the most constrained parts of the problem early.
pub struct MinRemainingValuesHeuristic;

impl<S: DomainSemantics> VariableSelectionHeuristic<S> for MinRemainingValuesHeuristic {
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId> {
        solution
            .domains
            .iter()
            .filter(|(_, domain)| domain.len() > 1)
            .min_by_key(|(_, domain)| domain.len())
            .map(|(var_id, _)| *var_id)
    }
}
