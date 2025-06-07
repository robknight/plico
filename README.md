# Plico: A Generic Constraint Solver in Rust

Plico is a generic, reusable constraint satisfaction problem (CSP) solver written in Rust. It is designed to be modular, using functional programming principles, and modern constraint programming techniques. The engine is problem-agnostic and can be used to model your own logic puzzles and optimization problems.

## Architecture

The solver is built on a two-layered architecture, separating the generic engine from the specific problem being solved:

- **Generic Solver Backend:** A problem-agnostic engine that provides core data structures (using persistent immutable collections from `im`), algorithms (AC-3 propagation and backtracking search), and a standard library of common constraints. It knows _how_ to solve a CSP but knows nothing about the specific domain.
- **Problem-Specific Frontend:** A concrete implementation that defines the specific variables, values, and constraints for a particular domain. This is achieved by implementing the `DomainSemantics` trait, which bridges the gap between the problem and the solver. See the Sudoku example to see how this works.

## Features

- **Worklist-based AC-3 Propagator:** Efficiently enforces arc consistency to prune the search space.
- **Backtracking Search:** Systematically searches for solutions in problems that are not solvable by propagation alone.
- **Standard Constraint Library:** A growing collection of reusable, generic constraints:
  - `AllDifferent`: Ensures all variables in a set have unique values.
  - `Equal`: Enforces that two variables must have the same value.
  - `NotEqual`: Enforces that two variables must have different values.
- **Generic Value System:** Includes a `StandardValue` enum (for integers, booleans, etc.) that can be composed into problem-specific domains, allowing standard constraints to work across different problems.

## Examples

The repository includes two example implementations that demonstrate how to use the solver:

1.  **Sudoku Solver:** A solver for classic 9x9 Sudoku puzzles.
2.  **Map Colouring:** A solver for the graph colouring problem, applied to a map of Australian territories.

## Usage

To build the project and run the standard tests for the included examples:

```bash
cargo build
cargo test
```

The project also includes long-running, thorough property-based tests that are ignored by default to keep the default test suite fast. To run all tests, including the ignored ones:

```bash
cargo test -- --ignored
```
