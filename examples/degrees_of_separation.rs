use std::sync::Arc;

use clap::Parser;
use im::HashMap;
use plico::{
    error::Result,
    solver::{
        constraint::Constraint,
        constraints::{
            equal::EqualConstraint, reified_and::ReifiedAndConstraint,
            reified_member_of::ReifiedMemberOfConstraint, reified_or::ReifiedOrConstraint,
        },
        engine::{SolverEngine, VariableId},
        heuristics::{value::IdentityValueHeuristic, variable::SelectFirstHeuristic},
        semantics::DomainSemantics,
        solution::{DomainRepresentation, HashSetDomain, Solution},
        value::StandardValue,
    },
};
use rand::prelude::*;

// 1. Define the problem-specific types
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Person(u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MyValue {
    Person(Person),
    Bool(bool),
}

impl From<StandardValue> for MyValue {
    fn from(sv: StandardValue) -> Self {
        match sv {
            StandardValue::Bool(b) => MyValue::Bool(b),
            _ => panic!("Unsupported conversion"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MyConstraint {
    Equal(EqualConstraint<MySemantics>),
    ReifiedAnd(ReifiedAndConstraint<MySemantics>),
    ReifiedOr(ReifiedOrConstraint<MySemantics>),
    ReifiedMemberOf(ReifiedMemberOfConstraint<MySemantics>),
}

#[derive(Debug, Clone)]
pub struct MySemantics;

// 2. Implement DomainSemantics
impl DomainSemantics for MySemantics {
    type Value = MyValue;
    type ConstraintDefinition = MyConstraint;
    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            MyConstraint::Equal(c) => Box::new(c.clone()),
            MyConstraint::ReifiedAnd(c) => Box::new(c.clone()),
            MyConstraint::ReifiedOr(c) => Box::new(c.clone()),
            MyConstraint::ReifiedMemberOf(c) => Box::new(c.clone()),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 100)]
    num_people: u32,

    #[arg(long, default_value_t = 5)]
    path_length: u32,

    #[arg(long, default_value_t = 0.1)]
    connection_density: f64,

    #[arg(long, default_value_t = 5)]
    max_depth: u32,
}

fn generate_graph(
    num_people: u32,
    path_length: u32,
    connection_density: f64,
) -> (
    Vec<Person>,
    Person,
    Person,
    std::collections::HashSet<Vec<MyValue>>,
    std::collections::HashSet<Vec<MyValue>>,
) {
    let mut rng = rand::thread_rng();
    let people: Vec<Person> = (0..num_people).map(Person).collect();

    let mut friends = std::collections::HashSet::new();
    let mut colleagues = std::collections::HashSet::new();

    // 1. Create the "golden path"
    let mut path_people = people.clone();
    path_people.shuffle(&mut rng);
    let golden_path: Vec<Person> = path_people.into_iter().take(path_length as usize).collect();

    for window in golden_path.windows(2) {
        let p1 = &window[0];
        let p2 = &window[1];
        if rng.gen::<bool>() {
            friends.insert(vec![
                MyValue::Person(p1.clone()),
                MyValue::Person(p2.clone()),
            ]);
        } else {
            colleagues.insert(vec![
                MyValue::Person(p1.clone()),
                MyValue::Person(p2.clone()),
            ]);
        }
    }

    // 2. Add random connections
    let num_connections = (num_people as f64 * num_people as f64 * connection_density) as usize;
    for _ in 0..num_connections {
        let p1 = people.choose(&mut rng).unwrap().clone();
        let p2 = people.choose(&mut rng).unwrap().clone();
        if p1 == p2 {
            continue;
        }

        if rng.gen::<bool>() {
            friends.insert(vec![
                MyValue::Person(p1.clone()),
                MyValue::Person(p2.clone()),
            ]);
        } else {
            colleagues.insert(vec![
                MyValue::Person(p1.clone()),
                MyValue::Person(p2.clone()),
            ]);
        }
    }

    let start_person = golden_path.first().unwrap().clone();
    let end_person = golden_path.last().unwrap().clone();

    (people, start_person, end_person, friends, colleagues)
}

fn all_people_domain(people: &[Person]) -> Box<dyn DomainRepresentation<MyValue>> {
    let values = people.iter().map(|p| MyValue::Person(p.clone())).collect();
    Box::new(HashSetDomain::new(values))
}

fn new_bool_var(
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<MyValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> VariableId {
    let id = var_id();
    domains.insert(
        id,
        Box::new(HashSetDomain::new(
            [MyValue::Bool(true), MyValue::Bool(false)]
                .iter()
                .cloned()
                .collect(),
        )),
    );
    id
}

fn main() -> Result<()> {
    let args = Args::parse();
    assert!(
        args.path_length <= args.num_people,
        "Path length cannot be greater than the number of people."
    );
    assert!(
        args.max_depth >= args.path_length,
        "Max depth must be at least the actual path length."
    );

    let (people, start_person, end_person, friends_data, colleagues_data) =
        generate_graph(args.num_people, args.path_length, args.connection_density);

    let mut domains = HashMap::new();
    let mut constraints: Vec<MyConstraint> = vec![];
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
            [MyValue::Person(start_person)].iter().cloned().collect(),
        )) as Box<dyn DomainRepresentation<_>>,
    );

    let end_node = var_id();
    domains.insert(
        end_node,
        Box::new(HashSetDomain::new(
            [MyValue::Person(end_person.clone())]
                .iter()
                .cloned()
                .collect(),
        )) as Box<dyn DomainRepresentation<_>>,
    );

    println!(
        "Finding path from {:?} to {:?} with max depth {}",
        domains[&start_node], domains[&end_node], args.max_depth
    );

    let (path_found_bool, path_constraints) = build_path_predicate(
        start_node,
        end_node,
        &friends_data,
        &colleagues_data,
        &people,
        args.max_depth,
        &mut domains,
        &mut var_id,
    );
    constraints.extend(path_constraints);

    let b_is_true = var_id();
    domains.insert(
        b_is_true,
        Box::new(HashSetDomain::new(
            [MyValue::Bool(true)].iter().cloned().collect(),
        )),
    );
    constraints.push(MyConstraint::Equal(EqualConstraint::new(
        path_found_bool,
        b_is_true,
    )));

    let semantics = Arc::new(MySemantics);
    let initial_solution = Solution {
        domains,
        semantics: semantics.clone(),
    };
    let built_constraints = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    let solver = SolverEngine::new(
        Box::new(SelectFirstHeuristic),
        Box::new(IdentityValueHeuristic),
    );

    let (solution, stats) = solver.solve(&built_constraints, initial_solution)?;
    println!("Stats: {:?}", stats);

    if let Some(solution) = solution {
        let final_end_val = solution
            .domains
            .get(&end_node)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        println!("Found path to: {:?}", final_end_val);
        assert_eq!(final_end_val, MyValue::Person(end_person));
    } else {
        println!("No path found.");
    }

    Ok(())
}

fn build_link_predicate(
    from: VariableId,
    to: VariableId,
    friends: &std::collections::HashSet<Vec<MyValue>>,
    colleagues: &std::collections::HashSet<Vec<MyValue>>,
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<MyValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> (VariableId, Vec<MyConstraint>) {
    let b_link = new_bool_var(domains, var_id);

    let b_is_friend = new_bool_var(domains, var_id);
    let b_is_colleague = new_bool_var(domains, var_id);

    let constraints = vec![
        MyConstraint::ReifiedOr(ReifiedOrConstraint::new(
            b_link,
            vec![b_is_friend, b_is_colleague],
        )),
        MyConstraint::ReifiedMemberOf(ReifiedMemberOfConstraint::new(
            b_is_friend,
            vec![from, to],
            friends.clone(),
        )),
        MyConstraint::ReifiedMemberOf(ReifiedMemberOfConstraint::new(
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
    friends: &std::collections::HashSet<Vec<MyValue>>,
    colleagues: &std::collections::HashSet<Vec<MyValue>>,
    people: &[Person],
    max_depth: u32,
    domains: &mut HashMap<VariableId, Box<dyn DomainRepresentation<MyValue>>>,
    var_id: &mut dyn FnMut() -> VariableId,
) -> (VariableId, Vec<MyConstraint>) {
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
    constraints.push(MyConstraint::ReifiedAnd(ReifiedAndConstraint::new(
        b_indirect_path,
        vec![b_link_sc, b_path_ce],
    )));

    let b_path = new_bool_var(domains, var_id);
    constraints.push(MyConstraint::ReifiedOr(ReifiedOrConstraint::new(
        b_path,
        vec![b_direct_link, b_indirect_path],
    )));

    (b_path, constraints)
}
