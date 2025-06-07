use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
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

// N-Queens problem definition copied from examples/n_queens.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NQueensValue {
    Std(StandardValue),
}

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

fn n_queens_problem_setup(
    n: usize,
) -> (
    Vec<Box<dyn Constraint<NQueensSemantics>>>,
    Solution<NQueensSemantics>,
) {
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
    constraints.push(NQueensConstraint::AllDifferent(
        AllDifferentConstraint::new(variables.clone()),
    ));

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

    (built_constraints, initial_solution)
}

fn n_queens_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("N-Queens");
    for n in [8, 10, 12].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            let (constraints, initial_solution) = n_queens_problem_setup(n);
            let solver = SolverEngine::new();
            b.iter(|| {
                solver
                    .solve(black_box(&constraints), black_box(initial_solution.clone()))
                    .unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, n_queens_benchmark);
criterion_main!(benches);
