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
    use std::sync::Arc;

    use im::HashMap;

    use crate::solver::{
        engine::SolverEngine,
        semantics::DomainSemantics,
        solution::{CandidateSolution, DomainRepresentation, HashSetDomain},
    };

    use super::{AllDifferentConstraint, SudokuSemantics, SudokuValue};

    #[test]
    fn test_sudoku_solver() {
        // Initialize the subscriber. This will print logs to the console.
        // `try_init` is used to prevent panic if it's initialized multiple times.
        let _ = tracing_subscriber::fmt::try_init();

        // A simple, solvable puzzle.
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

        // 1. Setup
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
        // Rows
        for row in &variables {
            constraints.push(AllDifferentConstraint::new(row.clone()));
        }
        // Columns
        for c in 0..9 {
            let col_vars = (0..9).map(|r| variables[r][c]).collect();
            constraints.push(AllDifferentConstraint::new(col_vars));
        }
        // Boxes
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

        // 2. Execution
        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        // 3. Verification
        assert!(result.is_ok());
        let solution = result.unwrap().unwrap();

        // Check a few key cells that must be solved.
        // puzzle[0][2] should be 4
        let cell_0_2 = solution.domains.get(&variables[0][2]).unwrap();
        assert!(cell_0_2.is_singleton());
        assert_eq!(cell_0_2.get_singleton_value(), Some(SudokuValue::Number(4)));

        // puzzle[2][3] should be 3
        let cell_2_3 = solution.domains.get(&variables[2][3]).unwrap();
        assert!(cell_2_3.is_singleton());
        assert_eq!(cell_2_3.get_singleton_value(), Some(SudokuValue::Number(3)));
    }

    #[test]
    fn test_unsolvable_sudoku() {
        let _ = tracing_subscriber::fmt::try_init();

        // An unsolvable puzzle (two 5s in the first row).
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

        // 1. Setup (same as the other test)
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
        // Rows
        for row in &variables {
            constraints.push(AllDifferentConstraint::new(row.clone()));
        }
        // Columns
        for c in 0..9 {
            let col_vars = (0..9).map(|r| variables[r][c]).collect();
            constraints.push(AllDifferentConstraint::new(col_vars));
        }
        // Boxes
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

        // 2. Execution
        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        // 3. Verification
        assert!(result.is_ok());
        let maybe_solution = result.unwrap();
        assert!(
            maybe_solution.is_none(),
            "Solver found a solution for an unsolvable puzzle"
        );
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
        use proptest::prelude::*;
        use std::sync::Arc;

        type Grid = [[i64; 9]; 9];

        // A known, valid, solved Sudoku grid to use as a seed.
        const SEED_GRID: Grid = [
            [5, 3, 4, 6, 7, 8, 9, 1, 2],
            [6, 7, 2, 1, 9, 5, 3, 4, 8],
            [1, 9, 8, 3, 4, 2, 5, 6, 7],
            [8, 5, 9, 7, 6, 1, 4, 2, 3],
            [4, 2, 6, 8, 5, 3, 7, 9, 1],
            [7, 1, 3, 9, 2, 4, 8, 5, 6],
            [9, 6, 1, 5, 3, 7, 2, 8, 4],
            [2, 8, 7, 4, 1, 9, 6, 3, 5],
            [3, 4, 5, 2, 8, 6, 1, 7, 9],
        ];

        // Swaps two numbers in the grid.
        fn relabel(grid: &mut Grid, a: i64, b: i64) {
            for row in grid.iter_mut() {
                for cell in row.iter_mut() {
                    if *cell == a {
                        *cell = b;
                    } else if *cell == b {
                        *cell = a;
                    }
                }
            }
        }

        // Swaps two rows within the same 3-row band.
        fn swap_rows(grid: &mut Grid, r1: usize, r2: usize) {
            grid.swap(r1, r2);
        }

        // Swaps two columns within the same 3-column band.
        fn swap_cols(grid: &mut Grid, c1: usize, c2: usize) {
            for row in grid.iter_mut() {
                row.swap(c1, c2);
            }
        }

        // Swaps two 3-row bands.
        fn swap_row_bands(grid: &mut Grid, b1: usize, b2: usize) {
            let b1_start = b1 * 3;
            let b2_start = b2 * 3;
            for i in 0..3 {
                grid.swap(b1_start + i, b2_start + i);
            }
        }

        // Swaps two 3-column bands.
        fn swap_col_bands(grid: &mut Grid, b1: usize, b2: usize) {
            let b1_start = b1 * 3;
            let b2_start = b2 * 3;
            for i in 0..3 {
                for row in grid.iter_mut() {
                    row.swap(b1_start + i, b2_start + i);
                }
            }
        }

        // Returns a strategy that generates a valid, solved Sudoku grid and a
        // puzzle derived from it by removing some cells.
        fn sudoku_puzzle_strategy() -> impl Strategy<Value = (Grid, Grid)> {
            // Strategy to generate a list of random transformations
            let transformations_strategy = proptest::collection::vec(
                prop_oneof![
                    // 0: Relabel
                    (1..=9i64, 1..=9i64)
                        .prop_filter("numbers must be distinct", |(a, b)| a != b)
                        .prop_map(|(a, b)| (0, a as usize, b as usize, 0)),
                    // 1: Swap rows in a band
                    (0..3usize, 0..3usize, 0..3usize)
                        .prop_filter("rows must be distinct", |(_, r1, r2)| r1 != r2)
                        .prop_map(|(band, r1, r2)| (1, band, r1, r2)),
                    // 2: Swap cols in a band
                    (0..3usize, 0..3usize, 0..3usize)
                        .prop_filter("cols must be distinct", |(_, c1, c2)| c1 != c2)
                        .prop_map(|(band, c1, c2)| (2, band, c1, c2)),
                    // 3: Swap row bands
                    (0..3usize, 0..3usize)
                        .prop_filter("bands must be distinct", |(b1, b2)| b1 != b2)
                        .prop_map(|(b1, b2)| (3, b1, b2, 0)),
                    // 4: Swap col bands
                    (0..3usize, 0..3usize)
                        .prop_filter("bands must be distinct", |(b1, b2)| b1 != b2)
                        .prop_map(|(b1, b2)| (4, b1, b2, 0)),
                ],
                20..=50, // Apply 20 to 50 transformations
            );

            transformations_strategy
                .prop_flat_map(|transformations| {
                    let mut solved_grid = SEED_GRID;
                    for t in transformations {
                        match t {
                            (0, a, b, _) => relabel(&mut solved_grid, a as i64, b as i64),
                            (1, band, r1, r2) => {
                                swap_rows(&mut solved_grid, band * 3 + r1, band * 3 + r2)
                            }
                            (2, band, c1, c2) => {
                                swap_cols(&mut solved_grid, band * 3 + c1, band * 3 + c2)
                            }
                            (3, b1, b2, _) => swap_row_bands(&mut solved_grid, b1, b2),
                            (4, b1, b2, _) => swap_col_bands(&mut solved_grid, b1, b2),
                            _ => unreachable!(),
                        }
                    }

                    // Now, create a strategy for poking holes in the solved grid.
                    // This correctly uses proptest's RNG.
                    let hole_coords = (0..9usize, 0..9usize);
                    let holes_strategy = proptest::collection::hash_set(hole_coords, 20..=60);

                    (Just(solved_grid), holes_strategy)
                })
                .prop_map(|(solved_grid, holes)| {
                    let mut puzzle_grid = solved_grid;
                    for (r, c) in holes {
                        puzzle_grid[r][c] = 0;
                    }
                    (puzzle_grid, solved_grid)
                })
        }

        pub fn grid_to_solution(
            grid: &Grid,
        ) -> (CandidateSolution<SudokuSemantics>, Vec<Vec<u32>>) {
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

        pub fn get_constraints(
            variables: &[Vec<u32>],
        ) -> Vec<AllDifferentConstraint<SudokuSemantics>> {
            let mut constraints = Vec::new();
            // Rows
            for row in variables {
                constraints.push(AllDifferentConstraint::new(row.clone()));
            }
            // Columns
            for c in 0..9 {
                let col_vars = (0..9).map(|r| variables[r][c]).collect();
                constraints.push(AllDifferentConstraint::new(col_vars));
            }
            // Boxes
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

        proptest! {
            #[test]
            fn can_solve_generated_puzzles((puzzle_grid, solution_key) in sudoku_puzzle_strategy()) {
                // 1. Setup
                let (mut solution, variables) = grid_to_solution(&puzzle_grid);
                let constraints = get_constraints(&variables);
                let semantics = Arc::new(SudokuSemantics);
                solution.semantics = semantics.clone();

                let built_constraints: Vec<_> = constraints
                    .iter()
                    .map(|c| semantics.build_constraint(c))
                    .collect();

                // 2. Execution
                let solver = SolverEngine::new();
                let result = solver.solve(&built_constraints, solution);

                // 3. Verification
                assert!(result.is_ok());
                let maybe_solution = result.unwrap();
                assert!(maybe_solution.is_some(), "Solver failed to find a solution");

                let solved_grid = solution_to_grid(&maybe_solution.unwrap(), &variables);

                // For debugging:
                if solved_grid != solution_key {
                    println!("Solver output grid:\n{:?}\n", solved_grid);
                    println!("Correct solution grid:\n{:?}\n", solution_key);
                }

                assert_eq!(solved_grid, solution_key, "Solver found an incorrect solution at {}", line!());
            }
        }
    }
}
