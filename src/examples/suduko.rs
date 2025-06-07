use im::HashSet;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint, engine::VariableId, semantics::DomainSemantics,
        solution::CandidateSolution,
    },
};

#[derive(Debug)]
pub struct SudokuSemantics;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SudokuValue {
    Number(i64),
}

#[derive(Debug, Clone)]
pub struct AllDifferentConstraint<S: DomainSemantics + std::fmt::Debug> {
    vars: Vec<VariableId>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> AllDifferentConstraint<S> {
    pub fn new(vars: Vec<VariableId>) -> Self {
        Self {
            vars,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for AllDifferentConstraint<S> {
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &CandidateSolution<S>,
    ) -> Result<Option<CandidateSolution<S>>> {
        // Find all values that are already fixed in other variables in this group.
        let mut fixed_values_to_remove = HashSet::new();
        for var in &self.vars {
            if *var != *target_var {
                if let Some(domain) = solution.domains.get(var) {
                    if domain.is_singleton() {
                        if let Some(fixed_value) = domain.get_singleton_value() {
                            fixed_values_to_remove.insert(fixed_value.clone());
                        }
                    }
                }
            }
        }

        if fixed_values_to_remove.is_empty() {
            return Ok(None);
        }

        // Now, remove those fixed values from the target's domain.
        if let Some(target_domain) = solution.domains.get(target_var) {
            let original_size = target_domain.len();
            let new_domain = target_domain.retain(&|val| !fixed_values_to_remove.contains(val));
            let changed = new_domain.len() < original_size;
            if changed {
                let new_domains = solution.domains.update(*target_var, new_domain);
                let new_solution = CandidateSolution {
                    domains: new_domains,
                    semantics: solution.semantics.clone(),
                };
                return Ok(Some(new_solution));
            }
        }

        Ok(None)
    }
}

impl DomainSemantics for SudokuSemantics {
    /// The concrete type for a value in a cell's domain.
    type Value = SudokuValue;

    /// The structure that defines a constraint for Sudoku.
    type ConstraintDefinition = AllDifferentConstraint<Self>;

    // These kind IDs are no longer relevant in the new design, but we'll leave
    // them for now until we refactor the trait itself.
    const NUMERIC_KIND_ID: u8 = 1;
    const POD_ID_KIND_ID: u8 = 0;
    const KEY_KIND_ID: u8 = 0;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        Box::new(AllDifferentConstraint::new(def.vars.clone()))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::sync::Arc;

    use im::HashMap;

    use crate::solver::{
        engine::SolverEngine,
        semantics::DomainSemantics,
        solution::{CandidateSolution, DomainRepresentation, HashSetDomain},
    };

    use super::{prop_tests, AllDifferentConstraint, SudokuSemantics, SudokuValue};

    #[test]
    fn test_sudoku_solver() {
        let _ = tracing_subscriber::fmt::try_init();

        let puzzle = [
            [5, 3, 0, 0, 7, 0, 0, 0, 0],
            [6, 0, 0, 1, 9, 5, 0, 0, 0],
            [0, 9, 8, 0, 0, 0, 0, 6, 0],
            [8, 0, 0, 0, 6, 0, 0, 0, 3],
            [4, 0, 0, 8, 0, 3, 0, 0, 1],
            [7, 0, 0, 0, 2, 0, 0, 0, 6],
            [0, 6, 0, 0, 0, 0, 2, 8, 0],
            [0, 0, 0, 4, 1, 9, 0, 0, 5],
            [0, 0, 0, 0, 8, 0, 0, 7, 9],
        ];

        let variables: Vec<Vec<_>> = (0..9)
            .map(|row| (0..9).map(|col| (row * 9 + col) as u32).collect())
            .collect();

        let mut domains = HashMap::new();
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                let value = puzzle[r][c];
                let domain_values = if value == 0 {
                    (1..=9).map(SudokuValue::Number).collect()
                } else {
                    [SudokuValue::Number(value)].iter().cloned().collect()
                };
                let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                    Box::new(HashSetDomain::new(domain_values));
                domains.insert(var_id, domain);
            }
        }

        let semantics = Arc::new(SudokuSemantics);
        let initial_solution = CandidateSolution {
            domains,
            semantics: semantics.clone(),
        };

        let mut constraints = Vec::new();
        for row in &variables {
            constraints.push(AllDifferentConstraint::new(row.clone()));
        }
        for c in 0..9 {
            let col_vars = (0..9).map(|r| variables[r][c]).collect();
            constraints.push(AllDifferentConstraint::new(col_vars));
        }
        for br in 0..3 {
            for bc in 0..3 {
                let box_vars = variables[(br * 3)..(br * 3 + 3)]
                    .iter()
                    .flat_map(|row| &row[(bc * 3)..(bc * 3 + 3)])
                    .cloned()
                    .collect();
                constraints.push(AllDifferentConstraint::new(box_vars));
            }
        }

        let built_constraints: Vec<_> = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect();

        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        assert!(result.is_ok());
        let solution = result.unwrap().unwrap();

        let cell_0_2 = solution.domains.get(&variables[0][2]).unwrap();
        assert!(cell_0_2.is_singleton());
        assert_eq!(cell_0_2.get_singleton_value(), Some(SudokuValue::Number(4)));

        let cell_2_3 = solution.domains.get(&variables[2][3]).unwrap();
        assert!(cell_2_3.is_singleton());
        assert_eq!(cell_2_3.get_singleton_value(), Some(SudokuValue::Number(3)));
    }

    #[test]
    fn test_unsolvable_sudoku() {
        let _ = tracing_subscriber::fmt::try_init();
        let puzzle = [
            [5, 3, 0, 0, 7, 0, 0, 0, 5], // Conflict here
            [6, 0, 0, 1, 9, 5, 0, 0, 0],
            [0, 9, 8, 0, 0, 0, 0, 6, 0],
            [8, 0, 0, 0, 6, 0, 0, 0, 3],
            [4, 0, 0, 8, 0, 3, 0, 0, 1],
            [7, 0, 0, 0, 2, 0, 0, 0, 6],
            [0, 6, 0, 0, 0, 0, 2, 8, 0],
            [0, 0, 0, 4, 1, 9, 0, 0, 5],
            [0, 0, 0, 0, 8, 0, 0, 7, 9],
        ];

        let variables: Vec<Vec<_>> = (0..9)
            .map(|row| (0..9).map(|col| (row * 9 + col) as u32).collect())
            .collect();

        let mut domains = HashMap::new();
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                let value = puzzle[r][c];
                let domain_values = if value == 0 {
                    (1..=9).map(SudokuValue::Number).collect()
                } else {
                    [SudokuValue::Number(value)].iter().cloned().collect()
                };
                let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                    Box::new(HashSetDomain::new(domain_values));
                domains.insert(var_id, domain);
            }
        }

        let semantics = Arc::new(SudokuSemantics);
        let initial_solution = CandidateSolution {
            domains,
            semantics: semantics.clone(),
        };

        let mut constraints = Vec::new();
        for row in &variables {
            constraints.push(AllDifferentConstraint::new(row.clone()));
        }
        for c in 0..9 {
            let col_vars = (0..9).map(|r| variables[r][c]).collect();
            constraints.push(AllDifferentConstraint::new(col_vars));
        }
        for br in 0..3 {
            for bc in 0..3 {
                let box_vars = variables[(br * 3)..(br * 3 + 3)]
                    .iter()
                    .flat_map(|row| &row[(bc * 3)..(bc * 3 + 3)])
                    .cloned()
                    .collect();
                constraints.push(AllDifferentConstraint::new(box_vars));
            }
        }

        let built_constraints: Vec<_> = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect();

        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        assert!(result.is_ok());
        let maybe_solution = result.unwrap();
        assert!(
            maybe_solution.is_none(),
            "Solver found a solution for an unsolvable puzzle"
        );
    }

    #[test]
    fn test_failing_proptest_case() {
        let _ = tracing_subscriber::fmt::try_init();

        let puzzle_grid: prop_tests::Grid = [
            [0, 0, 1, 4, 0, 0, 8, 0, 0],
            [3, 9, 2, 0, 8, 5, 7, 1, 4],
            [5, 0, 0, 0, 0, 0, 0, 3, 0],
            [0, 3, 0, 0, 1, 0, 4, 0, 0],
            [1, 0, 9, 5, 6, 0, 0, 8, 7],
            [4, 0, 7, 0, 3, 8, 9, 6, 1],
            [0, 0, 5, 0, 0, 0, 6, 7, 3],
            [0, 0, 8, 0, 9, 0, 5, 0, 2],
            [2, 0, 3, 0, 5, 6, 0, 9, 0],
        ];
        let solution_key: prop_tests::Grid = [
            [6, 7, 1, 4, 2, 3, 8, 5, 9],
            [3, 9, 2, 6, 8, 5, 7, 1, 4],
            [5, 8, 4, 1, 7, 9, 2, 3, 6],
            [8, 3, 6, 9, 1, 7, 4, 2, 5],
            [1, 2, 9, 5, 6, 4, 3, 8, 7],
            [4, 5, 7, 2, 3, 8, 9, 6, 1],
            [9, 1, 5, 8, 4, 2, 6, 7, 3],
            [7, 6, 8, 3, 9, 1, 5, 4, 2],
            [2, 4, 3, 7, 5, 6, 1, 9, 8],
        ];

        // 1. Setup
        let (mut solution, variables) = prop_tests::grid_to_solution(&puzzle_grid);
        let constraints = prop_tests::get_constraints(&variables);
        let semantics = std::sync::Arc::new(SudokuSemantics);
        solution.semantics = semantics.clone();

        let built_constraints: Vec<_> = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect();

        // 2. Execution
        let solver = crate::solver::engine::SolverEngine::new();
        let result = solver.solve(&built_constraints, solution);

        // 3. Verification
        assert!(result.is_ok());
        let maybe_solution = result.unwrap();
        assert!(maybe_solution.is_some(), "Solver failed to find a solution");

        let solved_grid = prop_tests::solution_to_grid(&maybe_solution.unwrap(), &variables);

        // For debugging:
        if !prop_tests::is_valid_solution(&puzzle_grid, &solved_grid) {
            println!("Solver output grid:\n{:?}\n", solved_grid);
            println!("Correct solution grid:\n{:?}\n", solution_key);
        }

        assert!(
            prop_tests::is_valid_solution(&puzzle_grid, &solved_grid),
            "Solver found an incorrect solution"
        );
    }
}

#[cfg(test)]
mod prop_tests {
    use super::{AllDifferentConstraint, SudokuSemantics, SudokuValue};
    use crate::solver::{
        engine::SolverEngine,
        semantics::DomainSemantics,
        solution::{CandidateSolution, DomainRepresentation, HashSetDomain},
    };
    use im::HashMap;
    use proptest::{
        prelude::*,
        strategy::{Just, NewTree, Strategy},
        test_runner::TestRunner,
    };
    use std::sync::Arc;
    use sudoku::Sudoku;

    pub type Grid = [[i64; 9]; 9];

    /// Converts a `sudoku` crate `[u8; 81]` representation to our `[[i64; 9]; 9]` grid.
    fn sudoku_bytes_to_grid(bytes: &[u8; 81]) -> Grid {
        let mut grid = [[0i64; 9]; 9];
        for i in 0..81 {
            grid[i / 9][i % 9] = bytes[i] as i64;
        }
        grid
    }

    #[derive(Debug, Clone)]
    struct SudokuGenerationStrategy;

    impl Strategy for SudokuGenerationStrategy {
        type Tree = <Just<(Grid, Grid)> as Strategy>::Tree;
        type Value = (Grid, Grid);

        fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
            let solved_sudoku = Sudoku::generate_solved_with_rng(runner.rng());

            let all_symmetries = [
                sudoku::Symmetry::VerticalMirror,
                sudoku::Symmetry::HorizontalMirror,
                sudoku::Symmetry::VerticalAndHorizontalMirror,
                sudoku::Symmetry::DiagonalMirror,
                sudoku::Symmetry::AntidiagonalMirror,
                sudoku::Symmetry::BidiagonalMirror,
                sudoku::Symmetry::QuarterRotation,
                sudoku::Symmetry::HalfRotation,
                sudoku::Symmetry::Dihedral,
                sudoku::Symmetry::None,
            ];
            let symmetry_index = (runner.rng().next_u64() % all_symmetries.len() as u64) as usize;
            let chosen_symmetry = all_symmetries[symmetry_index];

            let puzzle = Sudoku::generate_with_symmetry_and_rng_from(
                solved_sudoku,
                chosen_symmetry,
                runner.rng(),
            );

            let solved_grid = sudoku_bytes_to_grid(&solved_sudoku.to_bytes());
            let puzzle_grid = sudoku_bytes_to_grid(&puzzle.to_bytes());

            Just((puzzle_grid, solved_grid)).new_tree(runner)
        }
    }

    fn sudoku_puzzle_strategy() -> SudokuGenerationStrategy {
        SudokuGenerationStrategy
    }

    pub fn grid_to_solution(grid: &Grid) -> (CandidateSolution<SudokuSemantics>, Vec<Vec<u32>>) {
        let variables: Vec<Vec<_>> = (0..9)
            .map(|row| (0..9).map(|col| (row * 9 + col) as u32).collect())
            .collect();

        let mut domains = HashMap::new();
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                let value = grid[r][c];
                let domain_values = if value == 0 {
                    (1..=9).map(SudokuValue::Number).collect()
                } else {
                    [SudokuValue::Number(value)].iter().cloned().collect()
                };
                let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                    Box::new(HashSetDomain::new(domain_values));
                domains.insert(var_id, domain);
            }
        }

        let semantics = Arc::new(SudokuSemantics);
        let initial_solution = CandidateSolution { domains, semantics };

        (initial_solution, variables)
    }

    pub fn solution_to_grid(
        solution: &CandidateSolution<SudokuSemantics>,
        variables: &[Vec<u32>],
    ) -> Grid {
        let mut grid = [[0; 9]; 9];
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                if let Some(domain) = solution.domains.get(&var_id) {
                    if domain.is_singleton() {
                        if let Some(SudokuValue::Number(val)) = domain.get_singleton_value() {
                            grid[r][c] = val;
                        }
                    }
                }
            }
        }
        grid
    }

    pub fn get_constraints(variables: &[Vec<u32>]) -> Vec<AllDifferentConstraint<SudokuSemantics>> {
        let mut constraints = Vec::new();
        for row in variables {
            constraints.push(AllDifferentConstraint::new(row.clone()));
        }
        for c in 0..9 {
            let col_vars = (0..9).map(|r| variables[r][c]).collect();
            constraints.push(AllDifferentConstraint::new(col_vars));
        }
        for br in 0..3 {
            for bc in 0..3 {
                let box_vars = variables[(br * 3)..(br * 3 + 3)]
                    .iter()
                    .flat_map(|row| &row[(bc * 3)..(bc * 3 + 3)])
                    .cloned()
                    .collect();
                constraints.push(AllDifferentConstraint::new(box_vars));
            }
        }
        constraints
    }

    /// Verifies that a solved grid is a valid solution for a given puzzle.
    pub fn is_valid_solution(puzzle: &Grid, solution: &Grid) -> bool {
        // 1. Check that the solution respects the original puzzle's clues.
        for r in 0..9 {
            for c in 0..9 {
                if puzzle[r][c] != 0 && puzzle[r][c] != solution[r][c] {
                    return false;
                }
            }
        }

        // 2. Check that the solution is a valid Sudoku grid.
        for i in 0..9 {
            let mut row_digits = std::collections::HashSet::new();
            let mut col_digits = std::collections::HashSet::new();
            for j in 0..9 {
                if !row_digits.insert(solution[i][j]) || solution[i][j] == 0 {
                    return false;
                }
                if !col_digits.insert(solution[j][i]) || solution[j][i] == 0 {
                    return false;
                }
            }
        }

        for br in 0..3 {
            for bc in 0..3 {
                let mut box_digits = std::collections::HashSet::new();
                for r_offset in 0..3 {
                    for c_offset in 0..3 {
                        let r = br * 3 + r_offset;
                        let c = bc * 3 + c_offset;
                        if !box_digits.insert(solution[r][c]) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    proptest! {
        #[ignore]
        #[test]
        fn can_solve_generated_puzzles((puzzle_grid, solution_key) in sudoku_puzzle_strategy()) {
            let (mut solution, variables) = grid_to_solution(&puzzle_grid);
            let constraints = get_constraints(&variables);
            let semantics = Arc::new(SudokuSemantics);
            solution.semantics = semantics.clone();

            let built_constraints: Vec<_> = constraints
                .iter()
                .map(|c| semantics.build_constraint(c))
                .collect();

            let solver = SolverEngine::new();
            let result = solver.solve(&built_constraints, solution);

            assert!(result.is_ok());
            let maybe_solution = result.unwrap();
            assert!(maybe_solution.is_some(), "Solver failed to find a solution");

            let solved_grid = solution_to_grid(&maybe_solution.unwrap(), &variables);

            if !is_valid_solution(&puzzle_grid, &solved_grid) {
                println!("Puzzle grid:\n{:?}\n", puzzle_grid);
                println!("Solver output grid:\n{:?}\n", solved_grid);
                println!("Original solution grid:\n{:?}\n", solution_key);
            }

            assert!(
                is_valid_solution(&puzzle_grid, &solved_grid),
                "Solver found an invalid solution"
            );
        }
    }
}
