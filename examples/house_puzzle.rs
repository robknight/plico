use std::sync::Arc;

use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::{
        all_different::AllDifferentConstraint, boolean_or::BooleanOrConstraint,
        not_equal::NotEqualConstraint, reified_equal::ReifiedEqualConstraint,
    },
    engine::{SearchStats, SolverEngine, VariableId},
    semantics::DomainSemantics,
    solution::{DomainRepresentation, HashSetDomain, Solution},
    value::StandardValue,
};

// --- DOMAIN-SPECIFIC DEFINITIONS ---

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HouseValue {
    Color(String),
    Bool(bool),
}

impl From<StandardValue> for HouseValue {
    fn from(val: StandardValue) -> Self {
        match val {
            StandardValue::Bool(b) => HouseValue::Bool(b),
            _ => panic!("Cannot convert non-boolean StandardValue to HouseValue"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HouseConstraint {
    AllDifferent(AllDifferentConstraint<HouseSemantics>),
    NotEqual(NotEqualConstraint<HouseSemantics>),
    ReifiedEqual(ReifiedEqualConstraint<HouseSemantics>),
    BooleanOr(BooleanOrConstraint<HouseSemantics>),
}

#[derive(Debug, Clone)]
pub struct HouseSemantics;

impl DomainSemantics for HouseSemantics {
    type Value = HouseValue;
    type ConstraintDefinition = HouseConstraint;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            HouseConstraint::AllDifferent(c) => Box::new(c.clone()),
            HouseConstraint::NotEqual(c) => Box::new(c.clone()),
            HouseConstraint::ReifiedEqual(c) => Box::new(c.clone()),
            HouseConstraint::BooleanOr(c) => Box::new(c.clone()),
        }
    }
}

pub fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let (maybe_solution, stats) = solve_puzzle();
    if let Some(solution) = maybe_solution {
        println!("Solution found!");
        let alice_house = solution
            .domains
            .get(&0)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        let bob_house = solution
            .domains
            .get(&1)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        let carol_house = solution
            .domains
            .get(&2)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        println!("Alice lives in the {:?} house.", alice_house);
        println!("Bob lives in the {:?} house.", bob_house);
        println!("Carol lives in the {:?} house.", carol_house);
        println!("\nStats:\n{:#?}", stats);
    } else {
        println!("No solution found.");
    }
}

fn solve_puzzle() -> (Option<Solution<HouseSemantics>>, SearchStats) {
    // 1. DEFINE VARIABLES
    let alice: VariableId = 0;
    let bob: VariableId = 1;
    let carol: VariableId = 2;
    let people = vec![alice, bob, carol];

    let alice_is_green: VariableId = 3;
    let bob_is_blue: VariableId = 4;
    let carol_is_red: VariableId = 5;
    let or_vars = vec![alice_is_green, bob_is_blue, carol_is_red];

    // Constant value variables
    let red_house_var: VariableId = 6;
    let green_house_var: VariableId = 7;
    let blue_house_var: VariableId = 8;

    // 2. DEFINE DOMAINS
    let red = HouseValue::Color("Red".to_string());
    let green = HouseValue::Color("Green".to_string());
    let blue = HouseValue::Color("Blue".to_string());
    let colors = [red.clone(), green.clone(), blue.clone()];

    let bools = [HouseValue::Bool(true), HouseValue::Bool(false)];

    let mut domains: HashMap<VariableId, Box<dyn DomainRepresentation<HouseValue>>> =
        HashMap::new();

    for person_var in &people {
        domains.insert(
            *person_var,
            Box::new(HashSetDomain::new(colors.iter().cloned().collect())),
        );
    }
    for or_var in &or_vars {
        domains.insert(
            *or_var,
            Box::new(HashSetDomain::new(bools.iter().cloned().collect())),
        );
    }
    domains.insert(
        red_house_var,
        Box::new(HashSetDomain::new([red].iter().cloned().collect())),
    );
    domains.insert(
        green_house_var,
        Box::new(HashSetDomain::new([green].iter().cloned().collect())),
    );
    domains.insert(
        blue_house_var,
        Box::new(HashSetDomain::new([blue].iter().cloned().collect())),
    );

    // 3. DEFINE CONSTRAINTS
    let constraints = vec![
        // Clue 1: All friends live in different colored houses.
        HouseConstraint::AllDifferent(AllDifferentConstraint::new(people.clone())),
        // Clue 2: Alice does not live in the Red house.
        HouseConstraint::NotEqual(NotEqualConstraint::new(alice, red_house_var)),
        // Clue 3: Bob does not live in the Green house.
        HouseConstraint::NotEqual(NotEqualConstraint::new(bob, green_house_var)),
        // Clue 4: At least one of the following is true...
        HouseConstraint::ReifiedEqual(ReifiedEqualConstraint::new(
            alice_is_green,
            alice,
            green_house_var,
        )),
        HouseConstraint::ReifiedEqual(ReifiedEqualConstraint::new(
            bob_is_blue,
            bob,
            blue_house_var,
        )),
        HouseConstraint::ReifiedEqual(ReifiedEqualConstraint::new(
            carol_is_red,
            carol,
            red_house_var,
        )),
        HouseConstraint::BooleanOr(BooleanOrConstraint::new(or_vars)),
    ];

    // 4. SOLVE
    let semantics = Arc::new(HouseSemantics);
    let initial_solution = Solution {
        domains,
        semantics: semantics.clone(),
    };

    let built_constraints: Vec<_> = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect();

    let solver = SolverEngine::new();
    solver.solve(&built_constraints, initial_solution).unwrap()
}

#[cfg(test)]
mod tests {
    use plico::solver::engine::SearchStats;

    use super::*;

    #[test]
    fn test_house_puzzle() {
        let (solution, _stats) = solve_puzzle();
        let solution = solution.expect("The puzzle should have a solution");

        let alice_house = solution
            .domains
            .get(&0)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        let bob_house = solution
            .domains
            .get(&1)
            .unwrap()
            .get_singleton_value()
            .unwrap();
        let carol_house = solution
            .domains
            .get(&2)
            .unwrap()
            .get_singleton_value()
            .unwrap();

        // Expected solution:
        // Alice -> Green
        // Bob -> Red
        // Carol -> Blue
        assert_eq!(alice_house, HouseValue::Color("Green".to_string()));
        assert_eq!(bob_house, HouseValue::Color("Red".to_string()));
        assert_eq!(carol_house, HouseValue::Color("Blue".to_string()));
    }
}
