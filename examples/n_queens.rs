use std::sync::Arc;

use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::{
        abs_diff_not_equal::AbsoluteDifferenceNotEqualConstraint,
        all_different::AllDifferentConstraint,
    },
    engine::{SolverEngine, VariableId},
    semantics::DomainSemantics,
    solution::{DomainRepresentation, HashSetDomain, Solution},
    value::{StandardValue, ValueArithmetic},
};

// 1. Define the problem-specific types
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NQueensValue {
    Std(StandardValue),
}

// Implement From<StandardValue> to use in reified constraints etc.
impl From<StandardValue> for NQueensValue {
    fn from(v: StandardValue) -> Self {
        NQueensValue::Std(v)
    }
}

impl ValueArithmetic for NQueensValue {
    fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (NQueensValue::Std(a), NQueensValue::Std(b)) => NQueensValue::Std(a.add(b)),
        }
    }

    fn sub(&self, other: &Self) -> Self {
        match (self, other) {
            (NQueensValue::Std(a), NQueensValue::Std(b)) => NQueensValue::Std(a.sub(b)),
        }
    }

    fn abs(&self) -> Self {
        match self {
            NQueensValue::Std(a) => NQueensValue::Std(a.abs()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NQueensConstraint {
    AllDifferent(AllDifferentConstraint<NQueensSemantics>),
    AbsoluteDifferenceNotEqual(AbsoluteDifferenceNotEqualConstraint<NQueensSemantics>),
}

#[derive(Debug, Clone)]
pub struct NQueensSemantics;

// 2. Implement DomainSemantics
impl DomainSemantics for NQueensSemantics {
    type Value = NQueensValue;
    type ConstraintDefinition = NQueensConstraint;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            NQueensConstraint::AllDifferent(c) => Box::new(c.clone()),
            NQueensConstraint::AbsoluteDifferenceNotEqual(c) => Box::new(c.clone()),
        }
    }
}

fn main() {
    // 3. Parse command-line argument for N
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <N>", args[0]);
        std::process::exit(1);
    }
    let n: usize = args[1].parse().expect("N must be an integer");

    // 4. Construct the problem
    let variables: Vec<VariableId> = (0..n as u32).collect();

    let mut domains = HashMap::new();
    let domain_values: Vec<NQueensValue> = (0..n as i64)
        .map(|i| NQueensValue::Std(StandardValue::Int(i)))
        .collect();

    for &var_id in &variables {
        domains.insert(
            var_id,
            Box::new(HashSetDomain::new(domain_values.iter().cloned().collect()))
                as Box<dyn DomainRepresentation<_>>,
        );
    }

    let semantics = Arc::new(NQueensSemantics);
    let initial_solution = Solution {
        domains,
        semantics: semantics.clone(),
    };

    let mut constraints = vec![];

    // AllDifferent constraint for columns
    constraints.push(NQueensConstraint::AllDifferent(
        AllDifferentConstraint::new(variables.clone()),
    ));

    // Diagonal constraints
    for i in 0..n {
        for j in (i + 1)..n {
            let var1 = variables[i];
            let var2 = variables[j];
            let row_diff = (j - i) as i64;
            constraints.push(NQueensConstraint::AbsoluteDifferenceNotEqual(
                AbsoluteDifferenceNotEqualConstraint::new(
                    var1,
                    var2,
                    NQueensValue::Std(StandardValue::Int(row_diff)),
                ),
            ));
        }
    }

    let built_constraints = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    // 5. Solve
    println!("Solving N-Queens for N={}", n);
    let solver = SolverEngine::new();
    let (solution, stats) = solver.solve(&built_constraints, initial_solution).unwrap();

    println!("\nSearch statistics:\n{:#?}", stats);

    // 6. Print solution
    if let Some(sol) = solution {
        println!("\nFound a solution:");
        let mut board = vec![vec!['.'; n]; n];
        for (row, &var_id) in variables.iter().enumerate() {
            if let Some(domain) = sol.domains.get(&var_id) {
                if let Some(NQueensValue::Std(StandardValue::Int(col))) =
                    domain.get_singleton_value()
                {
                    board[row][col as usize] = 'Q';
                }
            }
        }
        for row in board {
            println!("{}", row.iter().collect::<String>());
        }
    } else {
        println!("\nNo solution found.");
    }
}
