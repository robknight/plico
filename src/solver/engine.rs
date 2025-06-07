use std::collections::HashMap;

use tracing::debug;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint,
        semantics::DomainSemantics,
        solution::{HashSetDomain, Solution},
        work_list::WorkList,
    },
};

pub type VariableId = u32;
pub type ConstraintId = usize;

/// The main engine for solving constraint satisfaction problems.
///
/// The `SolverEngine` is responsible for taking a problem definition—a set of
/// variables, their domains, and a list of constraints—and finding a solution
/// that satisfies all constraints.
///
/// It uses a combination of constraint propagation (the AC-3 algorithm) and
/// backtracking search to explore the solution space.
pub struct SolverEngine;

impl SolverEngine {
    /// Creates a new `SolverEngine`.
    pub fn new() -> Self {
        Self
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
    /// * `Ok(Some(solution))` if a complete solution is found.
    /// * `Ok(None)` if the problem is proven to be unsolvable.
    /// * `Err(error)` if an error occurs during the solving process.
    pub fn solve<S: DomainSemantics + std::fmt::Debug>(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        // First, run the propagation loop to establish arc consistency.
        let arc_consistent_solution = self.arc_consistency(constraints, initial_solution)?;

        // If propagation alone solved it or proved it unsolvable, we're done.
        let Some(solution) = arc_consistent_solution else {
            return Ok(None);
        };
        if solution.is_complete() {
            return Ok(Some(solution));
        }

        // Otherwise, start the search.
        self.search(constraints, solution)
    }

    fn search<S: DomainSemantics + std::fmt::Debug>(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        solution: Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        // Base case: If the solution is complete, we've found a valid assignment.
        if solution.is_complete() {
            return Ok(Some(solution));
        }

        // Variable selection: Pick a variable to branch on.
        let Some(var_to_branch) = solution.select_unassigned_variable() else {
            // This should not be reached if `is_complete` is false, but we handle it.
            return Ok(Some(solution));
        };

        let domain = solution.domains.get(&var_to_branch).unwrap().clone();

        // Value iteration: Try each value in the chosen variable's domain.
        for value in domain.iter() {
            // Create a new candidate solution with the variable assigned to the chosen value.
            let new_domain = Box::new(HashSetDomain::new(im::hashset! {value.clone()}));
            let new_domains = solution.domains.update(var_to_branch, new_domain);
            let guess_solution = Solution {
                domains: new_domains,
                semantics: solution.semantics.clone(),
            };

            // Propagate constraints with the new assignment.
            if let Some(propagated_solution) = self.arc_consistency(constraints, guess_solution)? {
                // If propagation succeeded, recurse.
                if let Some(found_solution) = self.search(constraints, propagated_solution)? {
                    return Ok(Some(found_solution));
                }
            }
            // If arc_consistency returned None, it's a contradiction, so we backtrack.
        }

        // If we've tried all values for this variable and found no solution, this path is a dead end.
        Ok(None)
    }

    /// Establishes arc-consistency using the AC-3 algorithm.
    fn arc_consistency<S: DomainSemantics + std::fmt::Debug>(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
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

            if let Some(new_solution) = constraint.revise(&target_var, &solution)? {
                let old_domain_size = solution.domains.get(&target_var).unwrap().len();
                let new_domain_size = new_solution.domains.get(&target_var).unwrap().len();

                if new_domain_size == 0 {
                    return Ok(None); // Inconsistent
                }

                if new_domain_size < old_domain_size {
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
        }

        debug!("Solver loop finished successfully");

        // If we reach here, the solution is arc-consistent.
        // Now, we need to perform a search to find a concrete solution.
        // (This part is not yet implemented)

        // For now, return the pruned domains.
        Ok(Some(solution))
    }
}

impl Default for SolverEngine {
    fn default() -> Self {
        Self::new()
    }
}
