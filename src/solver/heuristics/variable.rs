//! Heuristics for selecting which variable to branch on next during the search.

use crate::solver::{engine::VariableId, semantics::DomainSemantics, solution::Solution};

/// A trait for variable-selection heuristics.
///
/// Implementors of this trait define a strategy for choosing which unassigned
/// variable the solver should branch on next. A good heuristic can dramatically
/// improve solver performance.
pub trait VariableSelectionHeuristic<S: DomainSemantics> {
    /// Selects the next variable to be assigned.
    ///
    /// # Arguments
    ///
    /// * `solution`: The current state of the solution, including the domains of
    ///   all variables.
    ///
    /// # Returns
    ///
    /// * `Some(VariableId)` of the chosen variable, if there are unassigned
    ///   variables.
    /// * `None` if all variables are already assigned (their domains are singletons).
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId>;
}

/// A simple heuristic that selects the first unassigned variable it finds.
pub struct SelectFirstHeuristic;

impl<S: DomainSemantics> VariableSelectionHeuristic<S> for SelectFirstHeuristic {
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId> {
        solution
            .domains
            .iter()
            .filter(|(_, domain)| domain.len() > 1)
            // Select the one with the smallest variable ID to ensure determinism.
            .min_by_key(|(var_id, _)| *var_id)
            .map(|(var_id, _)| *var_id)
    }
}

/// A heuristic that selects the variable with the Minimum Remaining Values (MRV) in its domain.
/// This is a "fail-first" strategy, aiming to tackle the most constrained parts of the problem early.
pub struct MinimumRemainingValuesHeuristic;

impl<S: DomainSemantics> VariableSelectionHeuristic<S> for MinimumRemainingValuesHeuristic {
    fn select_variable(&self, solution: &Solution<S>) -> Option<VariableId> {
        solution
            .domains
            .iter()
            .filter(|(_, domain)| domain.len() > 1)
            .min_by(|(var_a, domain_a), (var_b, domain_b)| {
                // Primary criterion: domain length (ascending)
                // Secondary criterion: variable id (ascending, for tie-breaking)
                (domain_a.len(), *var_a).cmp(&(domain_b.len(), *var_b))
            })
            .map(|(var_id, _)| *var_id)
    }
}
