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

pub trait SearchStrategy<S: DomainSemantics> {
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)>;
}

pub struct BacktrackingSearch<S: DomainSemantics> {
    variable_heuristic: Box<dyn VariableSelectionHeuristic<S>>,
    value_heuristic: Box<dyn ValueOrderingHeuristic<S>>,
}

impl<S: DomainSemantics + std::fmt::Debug> BacktrackingSearch<S> {
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

        let domain = solution.domains.get(&var_to_branch).unwrap().clone();

        for value in self.value_heuristic.order_values(var_to_branch, &solution) {
            let new_domain = Box::new(HashSetDomain::new(im::hashset! {value}));
            let new_domains = solution.domains.update(var_to_branch, new_domain);
            let guess_solution = solution.clone_with_domains(new_domains);

            if let Some(propagated_solution) =
                self.arc_consistency(constraints, guess_solution, &mut stats)?
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

    pub fn arc_consistency(
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
                worklist.push_back(*var_id, constraint_id);
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

        Ok(Some(solution))
    }
}

impl<S: DomainSemantics + std::fmt::Debug> SearchStrategy<S> for BacktrackingSearch<S> {
    fn solve(
        &self,
        constraints: &[Box<dyn Constraint<S>>],
        initial_solution: Solution<S>,
    ) -> Result<(Option<Solution<S>>, SearchStats)> {
        let mut stats = SearchStats::default();
        let arc_consistent_solution =
            self.arc_consistency(constraints, initial_solution, &mut stats)?;

        let Some(solution) = arc_consistent_solution else {
            return Ok((None, stats));
        };
        if solution.is_complete() {
            return Ok((Some(solution), stats));
        }

        self.search(constraints, solution, stats)
    }
}

pub struct RestartingSearch<S: DomainSemantics> {
    inner_strategy: Box<dyn SearchStrategy<S>>,
    restart_policy: Box<dyn RestartPolicy>,
}

impl<S: DomainSemantics> RestartingSearch<S> {
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
