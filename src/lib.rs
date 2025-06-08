//! Plico is a generic, reusable constraint satisfaction problem (CSP) solver.
//!
//! The engine is problem-agnostic and can be used to model and solve a wide
//! variety of logic puzzles and optimization problems. The core idea is a
//! two-layered architecture: a generic solver backend and a problem-specific
//! frontend.
//!
//! # Core Concepts
//!
//! - **[`DomainSemantics`]**: A trait you implement to define the "what" of your problem:
//!   the variables, values, and constraints.
//! - **[`Constraint`]**: A trait representing a rule that must be satisfied. The crate
//!   provides a standard library of common constraints like [`EqualConstraint`] and
//!   [`AllDifferentConstraint`].
//! - **[`SolverEngine`]**: The main engine that takes your problem definition and solves it.
//!
//! # Example: A Simple 2-Variable Problem
//!
//! Here is a simple example of solving for `?A != ?B` where `?A` can be `1` or `2`,
//! and `?B` can only be `1`. The solver should deduce that `?A` must be `2`.
//!
//! ```
//! use plico::solver::engine::{SolverEngine, VariableId};
//! use plico::solver::semantics::DomainSemantics;
//! use plico::solver::solution::{Solution, DomainRepresentation, HashSetDomain};
//! use plico::solver::value::StandardValue;
//! use plico::solver::constraint::Constraint;
//! use plico::solver::constraints::not_equal::NotEqualConstraint;
//! use std::sync::Arc;
//! use im::HashMap;
//!
//! // 1. Define the problem-specific types
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! pub enum MyValue { Std(StandardValue) }
//!
//! #[derive(Debug, Clone)]
//! pub enum MyConstraint {
//!     NotEqual(NotEqualConstraint<MySemantics>)
//! }
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! pub struct MyMetadata;
//!
//! #[derive(Debug, Clone)]
//! pub struct MySemantics;
//!
//! // 2. Implement DomainSemantics to bridge the gap
//! impl DomainSemantics for MySemantics {
//!     type Value = MyValue;
//!     type ConstraintDefinition = MyConstraint;
//!     type VariableMetadata = MyMetadata;
//!     fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
//!         match def {
//!             MyConstraint::NotEqual(c) => Box::new(c.clone()),
//!         }
//!     }
//! }
//!
//! // 3. Define the problem instance
//! let a: VariableId = 0;
//! let b: VariableId = 1;
//!
//! let mut domains = HashMap::new();
//! domains.insert(a, Box::new(HashSetDomain::new([MyValue::Std(StandardValue::Int(1)), MyValue::Std(StandardValue::Int(2))].iter().cloned().collect())) as Box<dyn DomainRepresentation<_>>);
//! domains.insert(b, Box::new(HashSetDomain::new([MyValue::Std(StandardValue::Int(1))].iter().cloned().collect())) as Box<dyn DomainRepresentation<_>>);
//!
//! let semantics = Arc::new(MySemantics);
//! let initial_solution = Solution::new(domains.clone(), im::HashMap::new(), semantics.clone());
//!
//! let constraints = vec![MyConstraint::NotEqual(NotEqualConstraint::new(a, b))];
//! let built = constraints.iter().map(|c| semantics.build_constraint(c)).collect::<Vec<_>>();
//!
//! // 4. Solve!
//! use plico::solver::heuristics::{value::IdentityValueHeuristic, variable::SelectFirstHeuristic};
//! use plico::solver::strategy::BacktrackingSearch;
//! let strategy = Box::new(BacktrackingSearch::new(
//!     Box::new(SelectFirstHeuristic),
//!     Box::new(IdentityValueHeuristic),
//! ));
//! let solver = SolverEngine::new(strategy);
//! let (solution, _stats) = solver.solve(&built, initial_solution).unwrap();
//! let solution = solution.unwrap();
//!
//! let final_a_val = solution.domains.get(&a).unwrap().get_singleton_value().unwrap();
//! assert_eq!(final_a_val, MyValue::Std(StandardValue::Int(2)));
//!
//! // Example with restarts
//! use plico::solver::heuristics::{
//!     restart::{NoRestartPolicy, RestartAfterNBacktracks},
//!     variable::RandomVariableHeuristic,
//! };
//! use plico::solver::strategy::RestartingSearch;
//!
//! let inner_strategy = Box::new(BacktrackingSearch::<MySemantics>::new(
//!     Box::new(RandomVariableHeuristic), // Use random selection for restarts
//!     Box::new(IdentityValueHeuristic),
//! ));
//!
//! let restart_strategy = Box::new(RestartingSearch::new(
//!     inner_strategy,
//!     Box::new(NoRestartPolicy), // or RestartAfterNBacktracks { max_backtracks: 10 }
//! ));
//!
//! let solver_with_restarts = SolverEngine::new(restart_strategy);
//! let restarted_domains = domains.clone();
//! let initial_solution_for_restarts = Solution::new(restarted_domains, im::HashMap::new(), semantics.clone());
//! // let (restarted_solution, _stats) = solver_with_restarts.solve(&built, initial_solution_for_restarts).unwrap();
//!
//! ```
//!
pub mod error;
pub mod solver;
