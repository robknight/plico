use std::collections::HashMap;

use tracing::debug;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint,
        engine::{ConstraintId, SearchStats, VariableId},
        heuristics::{
            restart::RestartPolicy, value::ValueOrderingHeuristic,
            variable::VariableSelectionHeuristic,
        },
        semantics::DomainSemantics,
        solution::{HashSetDomain, Solution},
        work_list::WorkList,
    },
};

/// A type alias for a boxed [`SearchStrategy`].
pub type BoxedSearchStrategy<S> = Box<dyn SearchStrategy<S>>;

/// A trait for defining a search algorithm to be used by the [`SolverEngine`].
///
/// This allows for modular and composable search behaviors. Different strategies
/// can be implemented to provide standard backtracking, restarts, conflict-directed
/// backjumping, or other advanced search techniques.
pub trait SearchStrategy<S: DomainSemantics> {
    /// Attempts to find a solution to the given constraint problem.
    ///
    /// # Arguments
    ///
    /// * `constraints`: The constraints that must be satisfied.
    /// * `initial_solution`: The starting state of the problem.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing either a tuple with an `Option<Solution>` and
    /// [`SearchStats`], or an error.
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)>;

    /// Performs the core arc-consistency propagation loop (e.g., AC-3).
    ///
    /// This is provided as a default method so that different search strategies
    /// can reuse the same fundamental propagation logic.
    ///
    /// # Arguments
    ///
    /// * `constraints`: The set of constraints to enforce.
    /// * `initial_solution`: The solution state to start from.
    /// * `stats`: A mutable reference to [`SearchStats`] to record statistics.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing an `Option<Solution>`. It returns `Ok(None)` if
    /// a domain is wiped out, indicating an inconsistency. Otherwise, it returns
    /// `Ok(Some(new_solution))` with the pruned domains.
    fn propagate(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
        stats: &mut SearchStats,
    ) -> Result<Option<Solution<S>>> {
        let mut solution = initial_solution;

        let mut dependency_graph: HashMap<VariableId, Vec<ConstraintId>> = HashMap::new();
        for (i, constraint) in constraints.iter().enumerate() {
            for var_id in constraint.variables() {
                dependency_graph.entry(*var_id).or_default().push(i);
            }
        }

        let mut worklist = WorkList::new();
        for (constraint_id, constraint) in constraints.iter().enumerate() {
            for var_id in constraint.variables() {
                worklist.push_back(constraint.priority(), *var_id, constraint_id);
            }
        }

        while let Some((target_var, constraint_id)) = worklist.pop_front() {
            let constraint = &constraints[constraint_id];
            let constraint_stats = stats.constraint_stats.entry(constraint_id).or_default();

            let start_time = std::time::Instant::now();
            constraint_stats.revisions += 1;

            if let Some(new_solution) = constraint.revise(&target_var, &solution)? {
                let old_domain_size = solution.domains.get(&target_var).unwrap().len();
                let new_domain_size = new_solution.domains.get(&target_var).unwrap().len();

                if new_domain_size == 0 {
                    return Ok(None);
                }

                if new_domain_size < old_domain_size {
                    constraint_stats.prunings += 1;
                    solution = new_solution;

                    if let Some(dependent_constraints) = dependency_graph.get(&target_var) {
                        for &dep_constraint_id in dependent_constraints {
                            for &neighbor_var in constraints[dep_constraint_id].variables() {
                                if neighbor_var != target_var {
                                    let priority = constraints[dep_constraint_id].priority();
                                    worklist.push_back(priority, neighbor_var, dep_constraint_id);
                                }
                            }
                        }
                    }
                }
            }
            constraint_stats.time_spent_micros += start_time.elapsed().as_micros() as u64;
        }

        debug!("Solver loop finished successfully");

        Ok(Some(solution))
    }
}

/// A [`SearchStrategy`] that implements a standard, chronological backtracking
/// (or depth-first search) algorithm.
///
/// This strategy explores the search space by picking an unassigned variable,
/// assigning it a value, propagating constraints, and then recursively
/// searching for a solution. If a dead end is reached, it backtracks and
/// tries a different value. The process is guided by a variable selection
/// heuristic and a value ordering heuristic.
pub struct BacktrackingSearch<S: DomainSemantics> {
    variable_heuristic: Box<dyn VariableSelectionHeuristic<S>>,
    value_heuristic: Box<dyn ValueOrderingHeuristic<S>>,
}

impl<S: DomainSemantics + std::fmt::Debug> BacktrackingSearch<S> {
    /// Creates a new `BacktrackingSearch` strategy.
    ///
    /// # Arguments
    ///
    /// * `variable_heuristic`: The heuristic to use for selecting which variable to
    ///   branch on next.
    /// * `value_heuristic`: The heuristic to use for ordering the values to try
    ///   for the selected variable.
    pub fn new(
        variable_heuristic: Box<dyn VariableSelectionHeuristic<S>>,
        value_heuristic: Box<dyn ValueOrderingHeuristic<S>>,
    ) -> Self {
        Self {
            variable_heuristic,
            value_heuristic,
        }
    }

    fn search(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        solution: Solution<S>,
        mut stats: SearchStats,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        stats.nodes_visited += 1;

        if solution.is_complete() {
            return Ok((Some(solution), stats));
        }

        let Some(var_to_branch) = self.variable_heuristic.select_variable(&solution) else {
            return Ok((Some(solution), stats));
        };

        for value in self.value_heuristic.order_values(var_to_branch, &solution) {
            let new_domain = Box::new(HashSetDomain::new(im::hashset! {value}));
            let new_domains = solution.domains.update(var_to_branch, new_domain);
            let guess_solution = solution.clone_with_domains(new_domains);

            if let Some(propagated_solution) =
                self.propagate(constraints, guess_solution, &mut stats)?
            {
                let (found_solution, new_stats) =
                    self.search(constraints, propagated_solution, stats)?;
                stats = new_stats;
                if found_solution.is_some() {
                    return Ok((found_solution, stats));
                }
            }
            stats.backtracks += 1;
        }

        Ok((None, stats))
    }
}

impl<S: DomainSemantics + std::fmt::Debug> SearchStrategy<S> for BacktrackingSearch<S> {
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        let mut stats = SearchStats::default();
        let arc_consistent_solution = self.propagate(constraints, initial_solution, &mut stats)?;

        let Some(solution) = arc_consistent_solution else {
            return Ok((None, stats));
        };
        if solution.is_complete() {
            return Ok((Some(solution), stats));
        }

        self.search(constraints, solution, stats)
    }
}

/// A meta-[`SearchStrategy`] that wraps another strategy and implements a restart policy.
///
/// Restarts are a common technique in constraint solving to avoid getting stuck in a
/// difficult part of the search space. This strategy runs an inner search strategy
/// (like [`BacktrackingSearch`]) and, based on a [`RestartPolicy`], may interrupt
/// the search and start it again from the beginning.
///
/// This is most effective when the inner strategy has a random component (e.g.,
/// using a randomized heuristic), allowing it to explore a different search path
/// on each run. The `SearchStats` returned by this strategy will be an
/// aggregation of the stats from all runs.
pub struct RestartingSearch<S: DomainSemantics> {
    inner_strategy: Box<dyn SearchStrategy<S>>,
    restart_policy: Box<dyn RestartPolicy>,
}

impl<S: DomainSemantics> RestartingSearch<S> {
    /// Creates a new `RestartingSearch` strategy.
    ///
    /// # Arguments
    ///
    /// * `inner_strategy`: The [`SearchStrategy`] to run for each search attempt.
    /// * `restart_policy`: The policy that determines when to restart the search.
    pub fn new(
        inner_strategy: Box<dyn SearchStrategy<S>>,
        restart_policy: Box<dyn RestartPolicy>,
    ) -> Self {
        Self {
            inner_strategy,
            restart_policy,
        }
    }
}

impl<S: DomainSemantics> SearchStrategy<S> for RestartingSearch<S> {
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        let mut cumulative_stats = SearchStats::default();

        loop {
            let (solution, search_stats) = self
                .inner_strategy
                .solve(constraints, initial_solution.clone())?;

            cumulative_stats.nodes_visited += search_stats.nodes_visited;
            cumulative_stats.backtracks += search_stats.backtracks;
            for (id, stats) in &search_stats.constraint_stats {
                let s = cumulative_stats.constraint_stats.entry(*id).or_default();
                s.revisions += stats.revisions;
                s.prunings += stats.prunings;
                s.time_spent_micros += stats.time_spent_micros;
            }

            if solution.is_some() || !self.restart_policy.should_restart(&search_stats) {
                return Ok((solution, cumulative_stats));
            }
        }
    }
}

/// A search strategy that only performs the initial arc-consistency propagation.
///
/// This strategy runs the propagation loop to make the initial problem state
/// arc-consistent and then immediately returns the result without performing
/// any search or branching. It is useful for inspecting the effects of initial
/// constraint propagation.
#[derive(Debug, Clone, Default)]
pub struct PropagationOnlySearch;

impl PropagationOnlySearch {
    /// Creates a new `PropagationOnlySearch` strategy.
    pub fn new() -> Self {
        Self {}
    }

    /// Returns the strategy as a boxed trait object.
    pub fn boxed<S: DomainSemantics + 'static>() -> BoxedSearchStrategy<S> {
        Box::new(Self::new())
    }
}

impl<S: DomainSemantics> SearchStrategy<S> for PropagationOnlySearch {
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        let mut stats = SearchStats::default();
        let solution = self.propagate(constraints, initial_solution, &mut stats)?;
        Ok((solution, stats))
    }
}
