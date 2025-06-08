use std::sync::Arc;

use im::{ordset, HashMap, OrdSet};
use plico::solver::{
    constraint::Constraint,
    constraints::{all_different::AllDifferentConstraint, sum_of::SumOfConstraint},
    engine::{SolverEngine, VariableId},
    heuristics::{value::IdentityValueHeuristic, variable::SelectFirstHeuristic},
    semantics::DomainSemantics,
    solution::{Domain, OrderedDomain, Solution},
    strategy::BacktrackingSearch,
    value::{StandardValue, ValueArithmetic},
};

// 1. Define the problem-specific types
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MagicSquareValue(StandardValue);

impl From<StandardValue> for MagicSquareValue {
    fn from(sv: StandardValue) -> Self {
        MagicSquareValue(sv)
    }
}

impl ValueArithmetic for MagicSquareValue {
    fn add(&self, other: &Self) -> Self {
        MagicSquareValue(self.0.add(&other.0))
    }
    fn sub(&self, other: &Self) -> Self {
        MagicSquareValue(self.0.sub(&other.0))
    }
    fn abs(&self) -> Self {
        MagicSquareValue(self.0.abs())
    }
}

#[derive(Debug, Clone)]
pub enum MagicSquareConstraint {
    AllDifferent(AllDifferentConstraint<MagicSquareSemantics>),
    SumOf(SumOfConstraint<MagicSquareSemantics>),
}

#[derive(Debug, Clone)]
pub struct MagicSquareSemantics;

// 2. Implement DomainSemantics to bridge the gap
impl DomainSemantics for MagicSquareSemantics {
    type Value = MagicSquareValue;
    type ConstraintDefinition = MagicSquareConstraint;
    type VariableMetadata = ();
    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            MagicSquareConstraint::AllDifferent(c) => Box::new(c.clone()),
            MagicSquareConstraint::SumOf(c) => Box::new(c.clone()),
        }
    }
}

fn main() {
    // 3. Define the problem instance
    let cell_vars: Vec<VariableId> = (0..9).collect();
    let sum_var: VariableId = 9;

    let mut domains: HashMap<VariableId, Domain<MagicSquareValue>> = HashMap::new();
    let cell_domain_values: OrdSet<MagicSquareValue> = (1..=9)
        .map(|i| MagicSquareValue(StandardValue::Int(i)))
        .collect();
    let cell_domain = Box::new(OrderedDomain::new(cell_domain_values));

    for &var_id in &cell_vars {
        domains.insert(var_id, cell_domain.clone() as Domain<MagicSquareValue>);
    }
    domains.insert(
        sum_var,
        Box::new(OrderedDomain::new(
            ordset! {MagicSquareValue(StandardValue::Int(15))},
        )) as Domain<MagicSquareValue>,
    );

    let semantics = Arc::new(MagicSquareSemantics);
    let initial_solution = Solution::new(domains, HashMap::new(), semantics.clone());

    let mut constraints = vec![MagicSquareConstraint::AllDifferent(
        AllDifferentConstraint::new(cell_vars.clone()),
    )];

    // Rows
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 1, 2],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![3, 4, 5],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![6, 7, 8],
        sum_var,
    )));
    // Columns
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 3, 6],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![1, 4, 7],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![2, 5, 8],
        sum_var,
    )));
    // Diagonals
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 4, 8],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![2, 4, 6],
        sum_var,
    )));

    let built = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    // 4. Solve!
    println!("Solving Magic Square...");
    let solver = SolverEngine::new(Box::new(BacktrackingSearch::new(
        Box::new(SelectFirstHeuristic),
        Box::new(IdentityValueHeuristic),
    )));
    let (solution, stats) = solver.solve(&built, initial_solution).unwrap();
    let solution = solution.unwrap();

    println!("Solution found!");
    for i in 0..3 {
        for j in 0..3 {
            let var_id = i * 3 + j;
            let value = solution
                .domains
                .get(&var_id)
                .unwrap()
                .get_singleton_value()
                .unwrap();
            if let MagicSquareValue(StandardValue::Int(val)) = value {
                print!("{:>3}", val);
            }
        }
        println!();
    }
    println!("\nStats:\n{:#?}", stats);
}

#[test]
fn test_solve_magic_square() {
    // Define the problem instance
    let cell_vars: Vec<VariableId> = (0..9).collect();
    let sum_var: VariableId = 9;

    let mut domains: HashMap<VariableId, Domain<MagicSquareValue>> = HashMap::new();
    let cell_domain_values: OrdSet<MagicSquareValue> = (1..=9)
        .map(|i| MagicSquareValue(StandardValue::Int(i)))
        .collect();
    let cell_domain = Box::new(OrderedDomain::new(cell_domain_values));

    for &var_id in &cell_vars {
        domains.insert(var_id, cell_domain.clone() as Domain<MagicSquareValue>);
    }
    domains.insert(
        sum_var,
        Box::new(OrderedDomain::new(
            ordset! {MagicSquareValue(StandardValue::Int(15))},
        )) as Domain<MagicSquareValue>,
    );

    let semantics = Arc::new(MagicSquareSemantics);
    let initial_solution = Solution::new(domains, HashMap::new(), semantics.clone());

    let mut constraints = vec![MagicSquareConstraint::AllDifferent(
        AllDifferentConstraint::new(cell_vars.clone()),
    )];

    // Rows
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 1, 2],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![3, 4, 5],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![6, 7, 8],
        sum_var,
    )));
    // Columns
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 3, 6],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![1, 4, 7],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![2, 5, 8],
        sum_var,
    )));
    // Diagonals
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![0, 4, 8],
        sum_var,
    )));
    constraints.push(MagicSquareConstraint::SumOf(SumOfConstraint::new(
        vec![2, 4, 6],
        sum_var,
    )));

    let built = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    // Solve!
    let solver = SolverEngine::new(Box::new(BacktrackingSearch::new(
        Box::new(SelectFirstHeuristic),
        Box::new(IdentityValueHeuristic),
    )));
    let (solution, _stats) = solver.solve(&built, initial_solution).unwrap();
    let solution = solution.unwrap();

    // Verify the solution
    let mut values = vec![0i64; 9];
    let mut seen_numbers = std::collections::HashSet::new();
    for i in 0..9 {
        let value = solution
            .domains
            .get(&(i as VariableId))
            .unwrap()
            .get_singleton_value()
            .unwrap();
        if let MagicSquareValue(StandardValue::Int(val)) = value {
            values[i] = val;
            seen_numbers.insert(val);
        }
    }

    // Check if all numbers from 1-9 are used
    let expected_numbers: std::collections::HashSet<i64> = (1..=9).collect();
    assert_eq!(
        seen_numbers, expected_numbers,
        "The numbers in the square are not 1-9."
    );

    // Check sums
    let magic_sum = 15;
    // Rows
    assert_eq!(
        values[0] + values[1] + values[2],
        magic_sum,
        "Row 0 does not sum to 15."
    );
    assert_eq!(
        values[3] + values[4] + values[5],
        magic_sum,
        "Row 1 does not sum to 15."
    );
    assert_eq!(
        values[6] + values[7] + values[8],
        magic_sum,
        "Row 2 does not sum to 15."
    );
    // Columns
    assert_eq!(
        values[0] + values[3] + values[6],
        magic_sum,
        "Column 0 does not sum to 15."
    );
    assert_eq!(
        values[1] + values[4] + values[7],
        magic_sum,
        "Column 1 does not sum to 15."
    );
    assert_eq!(
        values[2] + values[5] + values[8],
        magic_sum,
        "Column 2 does not sum to 15."
    );
    // Diagonals
    assert_eq!(
        values[0] + values[4] + values[8],
        magic_sum,
        "Main diagonal does not sum to 15."
    );
    assert_eq!(
        values[2] + values[4] + values[6],
        magic_sum,
        "Anti-diagonal does not sum to 15."
    );
}
