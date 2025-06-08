use std::collections::HashMap;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint, semantics::DomainSemantics, solution::Solution,
        strategy::SearchStrategy,
    },
};

/// A numeric identifier for a single variable in the constraint problem.
pub type VariableId = u32;
/// A numeric identifier for a single constraint in the constraint problem.
pub type ConstraintId = usize;

/// Holds performance statistics for a single constraint.
#[derive(Debug, Default, Clone, Copy)]
pub struct PerConstraintStats {
    /// The number of times the `revise` method was called for this constraint.
    pub revisions: u64,
    /// The number of times this constraint successfully pruned a variable's domain.
    pub prunings: u64,
    /// The total time spent executing the `revise` method for this constraint, in microseconds.
    pub time_spent_micros: u64,
}

/// Holds statistics for the entire search process.
#[derive(Debug, Default)]
pub struct SearchStats {
    /// The total number of nodes (states) visited in the search tree.
    pub nodes_visited: u64,
    /// The total number of times the search backtracked.
    pub backtracks: u64,
    /// A map from [`ConstraintId`] to the performance statistics for that constraint.
    pub constraint_stats: HashMap<ConstraintId, PerConstraintStats>,
}

/// The main engine for solving constraint satisfaction problems.
///
/// The `SolverEngine` is responsible for orchestrating the search process. It
/// takes a problem definition—a set of variables, their domains, and a list of
/// constraints—and finds a solution by delegating to a configurable
/// [`SearchStrategy`].
pub struct SolverEngine<S: DomainSemantics> {
    strategy: Box<dyn SearchStrategy<S>>,
}

impl<S: DomainSemantics + std::fmt::Debug> SolverEngine<S> {
    /// Creates a new `SolverEngine` with the specified search strategy.
    ///
    /// The strategy defines the algorithm used to find a solution (e.g.,
    /// simple backtracking, restarts, etc.).
    pub fn new(strategy: Box<dyn SearchStrategy<S>>) -> Self {
        Self { strategy }
    }

    /// Attempts to solve the given constraint satisfaction problem.
    ///
    /// This method delegates the entire solving process to the [`SearchStrategy`]
    /// that was provided when the engine was created. The strategy will explore
    /// the search space to find a solution that satisfies all constraints.
    ///
    /// # Arguments
    ///
    /// * `constraints`: A slice of boxed [`Constraint`] trait objects that define the
    ///   rules of the problem.
    /// * `initial_solution`: A [`Solution`] representing the initial state
    ///   of the problem, including the initial domains for all variables.
    ///
    /// # Returns
    ///
    /// * `Ok((Some(solution), stats))` if a complete solution is found.
    /// * `Ok((None, stats))` if the problem is proven to be unsolvable.
    /// * `Err(error)` if an error occurs during the solving process.
    pub fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        self.strategy.solve(constraints, initial_solution)
    }
}
