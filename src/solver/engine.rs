use std::collections::HashMap;

use tracing::debug;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint, semantics::DomainSemantics, solution::Solution,
        strategy::SearchStrategy, work_list::WorkList,
    },
};

pub type VariableId = u32;
pub type ConstraintId = usize;

#[derive(Debug, Default, Clone, Copy)]
pub struct PerConstraintStats {
    pub revisions: u64,
    pub prunings: u64,
    pub time_spent_micros: u64,
}

#[derive(Debug, Default)]
pub struct SearchStats {
    pub nodes_visited: u64,
    pub backtracks: u64,
    pub constraint_stats: HashMap<ConstraintId, PerConstraintStats>,
}

/// The main engine for solving constraint satisfaction problems.
///
/// The `SolverEngine` is responsible for taking a problem definition—a set of
/// variables, their domains, and a list of constraints—and finding a solution
/// that satisfies all constraints.
///
/// It uses a combination of constraint propagation (the AC-3 algorithm) and
/// backtracking search to explore the solution space.
pub struct SolverEngine<S: DomainSemantics> {
    strategy: Box<dyn SearchStrategy<S>>,
}

impl<S: DomainSemantics + std::fmt::Debug> SolverEngine<S> {
    /// Creates a new `SolverEngine` with the specified heuristics.
    pub fn new(strategy: Box<dyn SearchStrategy<S>>) -> Self {
        Self { strategy }
    }

    /// Attempts to solve the given constraint satisfaction problem.
    ///
    /// This method first applies constraint propagation to achieve arc consistency,
    /// which prunes the domains of variables. If the problem is not solved by
    /// propagation alone, it proceeds with a backtracking search to find a
    /// complete assignment.
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

    /// Establishes arc-consistency using the AC-3 algorithm.
    pub fn arc_consistency(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
        stats: &mut SearchStats,
    ) -> Result<Option<Solution<S>>> {
        let mut solution = initial_solution;

        // Build the dependency graph.
        let mut dependency_graph: HashMap<VariableId, Vec<usize>> = HashMap::new();
        for (i, constraint) in constraints.iter().enumerate() {
            for var_id in constraint.variables() {
                dependency_graph.entry(*var_id).or_default().push(i);
            }
        }

        // Initialize the worklist with all arcs.
        let mut worklist = WorkList::new();
        for (constraint_id, constraint) in constraints.iter().enumerate() {
            for var_id in constraint.variables() {
                worklist.push_back(*var_id, constraint_id);
            }
        }

        // Main propagation loop (AC-3)
        while let Some((target_var, constraint_id)) = worklist.pop_front() {
            let constraint = &constraints[constraint_id];
            let constraint_stats = stats.constraint_stats.entry(constraint_id).or_default();

            let start_time = std::time::Instant::now();
            constraint_stats.revisions += 1;

            if let Some(new_solution) = constraint.revise(&target_var, &solution)? {
                let old_domain_size = solution.domains.get(&target_var).unwrap().len();
                let new_domain_size = new_solution.domains.get(&target_var).unwrap().len();

                if new_domain_size == 0 {
                    return Ok(None); // Inconsistent
                }

                if new_domain_size < old_domain_size {
                    constraint_stats.prunings += 1;
                    solution = new_solution;

                    // The domain of `target_var` has shrunk. We need to re-check all
                    // other constraints that involve `target_var`.
                    if let Some(dependent_constraints) = dependency_graph.get(&target_var) {
                        for &dep_constraint_id in dependent_constraints {
                            for &neighbor_var in constraints[dep_constraint_id].variables() {
                                if neighbor_var != target_var {
                                    worklist.push_back(neighbor_var, dep_constraint_id);
                                }
                            }
                        }
                    }
                }
            }
            constraint_stats.time_spent_micros += start_time.elapsed().as_micros() as u64;
        }

        debug!("Solver loop finished successfully");

        // If we reach here, the solution is arc-consistent.
        Ok(Some(solution))
    }
}
