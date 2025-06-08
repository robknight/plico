use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::{
        abs_diff_not_equal::AbsoluteDifferenceNotEqualConstraint,
        all_different::AllDifferentConstraint, equal::EqualConstraint,
        reified_and::ReifiedAndConstraint, reified_member_of::ReifiedMemberOfConstraint,
        reified_or::ReifiedOrConstraint,
    },
    engine::{SolverEngine, VariableId},
    heuristics::{
        restart::RestartAfterNBacktracks,
        value::IdentityValueHeuristic,
        variable::{
            MinimumRemainingValuesHeuristic, RandomVariableHeuristic, SelectFirstHeuristic,
        },
    },
    semantics::DomainSemantics,
    solution::{DomainRepresentation, HashSetDomain, Solution},
    strategy::{BacktrackingSearch, RestartingSearch, SearchStrategy},
    value::{StandardValue, ValueArithmetic},
};
use rand::prelude::*;

// --- Generic Benchmark Harness ---

pub trait BenchmarkProblem<S: DomainSemantics> {
    fn name(&self) -> String;
    fn setup(&self) -> (Vec<Box<dyn Constraint<S>>>, Solution<S>);
}

type StrategyFactory<S> = Box<dyn Fn() -> Box<dyn SearchStrategy<S>>>;

fn create_strategy_palette<S>() -> Vec<(&'static str, StrategyFactory<S>)>
where
    S: DomainSemantics + std::fmt::Debug + 'static,
    S::Value: 'static,
{
    vec![
        (
            "Backtracking (SelectFirst)",
            Box::new(|| {
                Box::new(BacktrackingSearch::new(
                    Box::new(SelectFirstHeuristic),
                    Box::new(IdentityValueHeuristic),
                ))
            }),
        ),
        (
            "Backtracking (MRV)",
            Box::new(|| {
                Box::new(BacktrackingSearch::new(
                    Box::new(MinimumRemainingValuesHeuristic),
                    Box::new(IdentityValueHeuristic),
                ))
            }),
        ),
        (
            "Restarting (Random, 50 backtracks)",
            Box::new(|| {
                let inner = BacktrackingSearch::new(
                    Box::new(RandomVariableHeuristic),
                    Box::new(IdentityValueHeuristic),
                );
                Box::new(RestartingSearch::new(
                    Box::new(inner),
                    Box::new(RestartAfterNBacktracks { max_backtracks: 50 }),
                ))
            }),
        ),
    ]
}

fn run_problem_benchmarks<S>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    problem: &dyn BenchmarkProblem<S>,
    strategy_palette: &[(&'static str, StrategyFactory<S>)],
) where
    S: DomainSemantics + std::fmt::Debug + 'static,
    S::Value: 'static,
{
    let (constraints, initial_solution) = problem.setup();

    for (strategy_name, strategy_factory) in strategy_palette {
        let solver = SolverEngine::new(strategy_factory());
        group.bench_function(*strategy_name, |b| {
            b.iter(|| {
                let (solution, _stats) = solver
                    .solve(black_box(&constraints), black_box(initial_solution.clone()))
                    .unwrap();
                // We assert here to ensure the solver is correct, not just fast.
                assert!(solution.is_some());
            })
        });
    }
}

// --- N-Queens Benchmark Setup ---

struct NQueensProblem {
    size: usize,
}

impl BenchmarkProblem<NQueensSemantics> for NQueensProblem {
    fn name(&self) -> String {
        format!("N-Queens (N={})", self.size)
    }

    fn setup(
        &self,
    ) -> (
        Vec<Box<dyn Constraint<NQueensSemantics>>>,
        Solution<NQueensSemantics>,
    ) {
        n_queens_problem_setup(self.size)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NQueensMetadata;

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
    type VariableMetadata = NQueensMetadata;

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
        variable_metadata: HashMap::new(),
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

fn n_queens_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("N-Queens Heuristics");

    let problem = NQueensProblem { size: 10 };
    let strategies = create_strategy_palette();
    run_problem_benchmarks(&mut group, &problem, &strategies);

    group.finish();
}

// --- Degrees of Separation Benchmark Setup ---

struct DegreesOfSeparationProblem {
    num_people: u32,
    path_length: u32,
    connection_density: f64,
    max_depth: u32,
}

impl BenchmarkProblem<DoSSemantics> for DegreesOfSeparationProblem {
    fn name(&self) -> String {
        format!(
            "DoS (people={}, density={})",
            self.num_people, self.connection_density
        )
    }

    fn setup(
        &self,
    ) -> (
        Vec<Box<dyn Constraint<DoSSemantics>>>,
        Solution<DoSSemantics>,
    ) {
        let (people, start_person, end_person, friends_data, colleagues_data) =
            generate_graph(self.num_people, self.path_length, self.connection_density);
        let mut domains = HashMap::new();
        let mut constraints: Vec<DoSConstraint> = vec![];
        let mut next_var_id: VariableId = 0;
        let mut var_id = || {
            let id = next_var_id;
            next_var_id += 1;
            id
        };
        let start_node = var_id();
        domains.insert(
            start_node,
            Box::new(HashSetDomain::new(
                [DoSValue::Person(start_person)].iter().cloned().collect(),
            )) as Box<dyn DomainRepresentation<_>>,
        );
        let end_node = var_id();
        domains.insert(
            end_node,
            Box::new(HashSetDomain::new(
                [DoSValue::Person(end_person.clone())]
                    .iter()
                    .cloned()
                    .collect(),
            )) as Box<dyn DomainRepresentation<_>>,
        );
        let (path_found_bool, path_constraints) = build_path_predicate(
            start_node,
            end_node,
            &friends_data,
            &colleagues_data,
            &people,
            self.max_depth,
            &mut domains,
            &mut var_id,
        );
        constraints.extend(path_constraints);
        let b_is_true = var_id();
        domains.insert(
            b_is_true,
            Box::new(HashSetDomain::new(
                [DoSValue::Bool(true)].iter().cloned().collect(),
            )),
        );
        constraints.push(DoSConstraint::Equal(EqualConstraint::new(
            path_found_bool,
            b_is_true,
        )));
        let semantics = Arc::new(DoSSemantics);
        let initial_solution = Solution::new(domains, HashMap::new(), semantics.clone());
        let built_constraints = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect::<Vec<_>>();
        (built_constraints, initial_solution)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Person(u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DoSValue {
    Person(Person),
    Bool(bool),
}

impl From<StandardValue> for DoSValue {
    fn from(sv: StandardValue) -> Self {
        match sv {
            StandardValue::Bool(b) => DoSValue::Bool(b),
            _ => panic!("Unsupported conversion"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DoSMetadata;

#[derive(Debug, Clone)]
pub enum DoSConstraint {
    Equal(EqualConstraint<DoSSemantics>),
    ReifiedAnd(ReifiedAndConstraint<DoSSemantics>),
    ReifiedOr(ReifiedOrConstraint<DoSSemantics>),
    ReifiedMemberOf(ReifiedMemberOfConstraint<DoSSemantics>),
}

#[derive(Debug, Clone)]
pub struct DoSSemantics;

impl DomainSemantics for DoSSemantics {
    type Value = DoSValue;
    type ConstraintDefinition = DoSConstraint;
    type VariableMetadata = DoSMetadata;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            DoSConstraint::Equal(c) => Box::new(c.clone()),
            DoSConstraint::ReifiedAnd(c) => Box::new(c.clone()),
            DoSConstraint::ReifiedOr(c) => Box::new(c.clone()),
            DoSConstraint::ReifiedMemberOf(c) => Box::new(c.clone()),
        }
    }
}

fn generate_graph(
    num_people: u32,
    path_length: u32,
    connection_density: f64,
) -> (
    Vec<Person>,
    Person,
    Person,
    std::collections::HashSet<Vec<DoSValue>>,
    std::collections::HashSet<Vec<DoSValue>>,
) {
    let mut rng = rand::thread_rng();
    let people: Vec<Person> = (0..num_people).map(Person).collect();
    let mut friends = std::collections::HashSet::new();
    let mut colleagues = std::collections::HashSet::new();

    let mut path_people = people.clone();
    path_people.shuffle(&mut rng);
    let golden_path: Vec<Person> = path_people.into_iter().take(path_length as usize).collect();

    for window in golden_path.windows(2) {
        let p1 = &window[0];
        let p2 = &window[1];
        if rng.gen::<bool>() {
            friends.insert(vec![
                DoSValue::Person(p1.clone()),
                DoSValue::Person(p2.clone()),
            ]);
        } else {
            colleagues.insert(vec![
                DoSValue::Person(p1.clone()),
                DoSValue::Person(p2.clone()),
            ]);
        }
    }

    let num_connections = (num_people as f64 * num_people as f64 * connection_density) as usize;
    for _ in 0..num_connections {
        let p1 = people.choose(&mut rng).unwrap().clone();
        let p2 = people.choose(&mut rng).unwrap().clone();
        if p1 == p2 {
            continue;
        }
        if rng.gen::<bool>() {
            friends.insert(vec![
                DoSValue::Person(p1.clone()),
                DoSValue::Person(p2.clone()),
            ]);
        } else {
            colleagues.insert(vec![
                DoSValue::Person(p1.clone()),
                DoSValue::Person(p2.clone()),
            ]);
        }
    }

    let start_person = golden_path.first().unwrap().clone();
    let end_person = golden_path.last().unwrap().clone();
    (people, start_person, end_person, friends, colleagues)
}

fn all_people_domain(people: &[Person]) -> Box<dyn DomainRepresentation<DoSValue>> {
    let values = people.iter().map(|p| DoSValue::Person(p.clone())).collect();
    Box::new(HashSetDomain::new(values))
}

fn new_bool_var(
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<DoSValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> VariableId {
    let id = var_id();
    domains.insert(
        id,
        Box::new(HashSetDomain::new(
            [DoSValue::Bool(true), DoSValue::Bool(false)]
                .iter()
                .cloned()
                .collect(),
        )),
    );
    id
}

fn build_link_predicate(
    from: VariableId,
    to: VariableId,
    friends: &std::collections::HashSet<Vec<DoSValue>>,
    colleagues: &std::collections::HashSet<Vec<DoSValue>>,
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<DoSValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> (VariableId, Vec<DoSConstraint>) {
    let b_link = new_bool_var(domains, var_id);
    let b_is_friend = new_bool_var(domains, var_id);
    let b_is_colleague = new_bool_var(domains, var_id);

    let constraints = vec![
        DoSConstraint::ReifiedOr(ReifiedOrConstraint::new(
            b_link,
            vec![b_is_friend, b_is_colleague],
        )),
        DoSConstraint::ReifiedMemberOf(ReifiedMemberOfConstraint::new(
            b_is_friend,
            vec![from, to],
            friends.clone(),
        )),
        DoSConstraint::ReifiedMemberOf(ReifiedMemberOfConstraint::new(
            b_is_colleague,
            vec![from, to],
            colleagues.clone(),
        )),
    ];
    (b_link, constraints)
}

#[allow(clippy::too_many_arguments)]
fn build_path_predicate(
    start: VariableId,
    end: VariableId,
    friends: &std::collections::HashSet<Vec<DoSValue>>,
    colleagues: &std::collections::HashSet<Vec<DoSValue>>,
    people: &[Person],
    max_depth: u32,
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<DoSValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> (VariableId, Vec<DoSConstraint>) {
    if max_depth == 0 {
        return build_link_predicate(start, end, friends, colleagues, domains, var_id);
    }
    let (b_direct_link, mut constraints) =
        build_link_predicate(start, end, friends, colleagues, domains, var_id);
    let intermediate_node = var_id();
    domains.insert(intermediate_node, all_people_domain(people));
    let (b_link_sc, link_sc_constraints) = build_link_predicate(
        start,
        intermediate_node,
        friends,
        colleagues,
        domains,
        var_id,
    );
    constraints.extend(link_sc_constraints);
    let (b_path_ce, path_ce_constraints) = build_path_predicate(
        intermediate_node,
        end,
        friends,
        colleagues,
        people,
        max_depth - 1,
        domains,
        var_id,
    );
    constraints.extend(path_ce_constraints);
    let b_indirect_path = new_bool_var(domains, var_id);
    constraints.push(DoSConstraint::ReifiedAnd(ReifiedAndConstraint::new(
        b_indirect_path,
        vec![b_link_sc, b_path_ce],
    )));
    let b_path = new_bool_var(domains, var_id);
    constraints.push(DoSConstraint::ReifiedOr(ReifiedOrConstraint::new(
        b_path,
        vec![b_direct_link, b_indirect_path],
    )));
    (b_path, constraints)
}

fn degrees_of_separation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Degrees of Separation");

    let path_length = 3;
    let max_depth = 3;

    for num_people in [10, 20].iter() {
        for density in [0.05, 0.1].iter() {
            let problem = DegreesOfSeparationProblem {
                num_people: *num_people,
                path_length,
                connection_density: *density,
                max_depth,
            };

            let strategies = create_strategy_palette();
            let (constraints, initial_solution) = problem.setup();

            for (strategy_name, strategy_factory) in strategies {
                let id = format!("{}, {}", problem.name(), strategy_name);
                group.bench_function(id, |b| {
                    let solver = SolverEngine::new(strategy_factory());
                    b.iter(|| {
                        let (solution, _stats) = solver
                            .solve(black_box(&constraints), black_box(initial_solution.clone()))
                            .unwrap();
                        assert!(solution.is_some());
                    })
                });
            }
        }
    }
    group.finish();
}

fn solver_benchmarks(c: &mut Criterion) {
    n_queens_benchmarks(c);
    degrees_of_separation_benchmark(c);
}

criterion_group!(benches, solver_benchmarks);
criterion_main!(benches);
