use std::sync::Arc;

use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::all_different::AllDifferentConstraint,
    engine::{SearchStats, SolverEngine},
    semantics::DomainSemantics,
    solution::{DomainRepresentation, HashSetDomain, Solution},
    value::StandardValue,
};

#[derive(Debug)]
pub struct SudokuSemantics;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SudokuValue {
    Std(StandardValue),
}

impl DomainSemantics for SudokuSemantics {
    /// The concrete type for a value in a cell's domain.
    type Value = SudokuValue;

    /// The structure that defines a constraint for Sudoku.
    type ConstraintDefinition = AllDifferentConstraint<Self>;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        Box::new(AllDifferentConstraint::new(def.vars.clone()))
    }
}

/// Solves a hardcoded Sudoku puzzle and returns the solution and variable map.
pub fn solve_hardcoded_puzzle() -> (
    Option<Solution<SudokuSemantics>>,
    SearchStats,
    Vec<Vec<u32>>,
) {
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
                (1..=9)
                    .map(|v| SudokuValue::Std(StandardValue::Int(v)))
                    .collect()
            } else {
                [SudokuValue::Std(StandardValue::Int(value))]
                    .iter()
                    .cloned()
                    .collect()
            };
            let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                Box::new(HashSetDomain::new(domain_values));
            domains.insert(var_id, domain);
        }
    }

    let semantics = Arc::new(SudokuSemantics);
    let initial_solution = Solution {
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
    let (solution, stats) = solver.solve(&built_constraints, initial_solution).unwrap();
    (solution, stats, variables)
}

fn print_grid(solution: &Solution<SudokuSemantics>, variables: &[Vec<u32>]) {
    for r in 0..9 {
        if r % 3 == 0 && r != 0 {
            println!("- - - + - - - + - - -");
        }
        for c in 0..9 {
            if c % 3 == 0 && c != 0 {
                print!("| ");
            }
            let var_id = variables[r][c];
            let value = solution
                .domains
                .get(&var_id)
                .unwrap()
                .get_singleton_value()
                .map(|v| match v {
                    SudokuValue::Std(StandardValue::Int(i)) => i.to_string(),
                    _ => ".".to_string(),
                })
                .unwrap_or_else(|| ".".to_string());
            print!("{} ", value);
        }
        println!();
    }
}

pub fn main() {
    tracing_subscriber::fmt::init();
    println!("Solving hardcoded Sudoku puzzle...");
    let (maybe_solution, stats, variables) = solve_hardcoded_puzzle();
    if let Some(solution) = maybe_solution {
        println!("Solution found!");
        print_grid(&solution, &variables);
        println!("\nStats:\n{:#?}", stats);
    } else {
        println!("No solution found for the hardcoded puzzle.");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use im::HashMap;
    use plico::solver::{
        constraints::all_different::AllDifferentConstraint,
        engine::{SearchStats, SolverEngine},
        semantics::DomainSemantics,
        solution::{DomainRepresentation, HashSetDomain, Solution},
        value::StandardValue,
    };
    use pretty_assertions::assert_eq;

    use super::{prop_tests, solve_hardcoded_puzzle, SudokuSemantics, SudokuValue};

    #[test]
    fn test_sudoku_solver() {
        let _ = tracing_subscriber::fmt::try_init();

        let (solution, _stats, variables) = solve_hardcoded_puzzle();
        let solution = solution.unwrap();

        let cell_0_2 = solution.domains.get(&variables[0][2]).unwrap();
        assert!(cell_0_2.is_singleton());
        assert_eq!(
            cell_0_2.get_singleton_value(),
            Some(SudokuValue::Std(StandardValue::Int(4)))
        );

        let cell_2_3 = solution.domains.get(&variables[2][3]).unwrap();
        assert!(cell_2_3.is_singleton());
        assert_eq!(
            cell_2_3.get_singleton_value(),
            Some(SudokuValue::Std(StandardValue::Int(3)))
        );
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
                    (1..=9)
                        .map(|v| SudokuValue::Std(StandardValue::Int(v)))
                        .collect()
                } else {
                    [SudokuValue::Std(StandardValue::Int(value))]
                        .iter()
                        .cloned()
                        .collect()
                };
                let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                    Box::new(HashSetDomain::new(domain_values));
                domains.insert(var_id, domain);
            }
        }

        let semantics = Arc::new(SudokuSemantics);
        let initial_solution = Solution {
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
        let (solution, _stats) = solver.solve(&built_constraints, initial_solution).unwrap();
        assert!(solution.is_none());
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
        let solver = plico::solver::engine::SolverEngine::new();
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
    use std::sync::Arc;

    use im::HashMap;
    use plico::solver::{
        constraints::all_different::AllDifferentConstraint,
        engine::SolverEngine,
        semantics::DomainSemantics,
        solution::{DomainRepresentation, HashSetDomain, Solution},
        value::StandardValue,
    };
    use proptest::{
        prelude::*,
        strategy::{Just, NewTree, Strategy},
        test_runner::TestRunner,
    };
    use sudoku::Sudoku;

    use super::{SudokuSemantics, SudokuValue};

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

    pub fn grid_to_solution(grid: &Grid) -> (Solution<SudokuSemantics>, Vec<Vec<u32>>) {
        let variables: Vec<Vec<_>> = (0..9)
            .map(|row| (0..9).map(|col| (row * 9 + col) as u32).collect())
            .collect();

        let mut domains = HashMap::new();
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                let value = grid[r][c];
                let domain_values = if value == 0 {
                    (1..=9)
                        .map(|v| SudokuValue::Std(StandardValue::Int(v)))
                        .collect()
                } else {
                    [SudokuValue::Std(StandardValue::Int(value))]
                        .iter()
                        .cloned()
                        .collect()
                };
                let domain: Box<dyn DomainRepresentation<SudokuValue>> =
                    Box::new(HashSetDomain::new(domain_values));
                domains.insert(var_id, domain);
            }
        }

        let semantics = Arc::new(SudokuSemantics);
        let initial_solution = Solution { domains, semantics };

        (initial_solution, variables)
    }

    pub fn solution_to_grid(solution: &Solution<SudokuSemantics>, variables: &[Vec<u32>]) -> Grid {
        let mut grid = [[0; 9]; 9];
        for r in 0..9 {
            for c in 0..9 {
                let var_id = variables[r][c];
                if let Some(domain) = solution.domains.get(&var_id) {
                    if domain.is_singleton() {
                        if let Some(SudokuValue::Std(StandardValue::Int(val))) =
                            domain.get_singleton_value()
                        {
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
