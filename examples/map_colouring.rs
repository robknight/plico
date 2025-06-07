use std::{clone::Clone, sync::Arc};

use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::{equal::EqualConstraint, not_equal::NotEqualConstraint},
    engine::{SolverEngine, VariableId},
    semantics::DomainSemantics,
    solution::{DomainRepresentation, HashSetDomain, Solution},
    value::StandardValue,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapColouringValue {
    Std(StandardValue),
}

#[derive(Debug, Clone)]
pub enum MapColouringConstraint {
    NotEqual(NotEqualConstraint<MapColouringSemantics>),
    Equal(EqualConstraint<MapColouringSemantics>),
}

#[derive(Debug, Clone)]
pub struct MapColouringSemantics;

impl DomainSemantics for MapColouringSemantics {
    type Value = MapColouringValue;
    type ConstraintDefinition = MapColouringConstraint;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            MapColouringConstraint::NotEqual(c) => Box::new((*c).clone()),
            MapColouringConstraint::Equal(c) => Box::new((*c).clone()),
        }
    }
}

pub fn create_problem() -> (Solution<MapColouringSemantics>, Vec<MapColouringConstraint>) {
    let wa: VariableId = 0;
    let nt: VariableId = 1;
    let sa: VariableId = 2;
    let q: VariableId = 3;
    let nsw: VariableId = 4;
    let v: VariableId = 5;
    let t: VariableId = 6;

    let red = MapColouringValue::Std(StandardValue::Int(0));
    let green = MapColouringValue::Std(StandardValue::Int(1));
    let blue = MapColouringValue::Std(StandardValue::Int(2));

    let domains = (0..=t)
        .map(|id| {
            (
                id,
                Box::new(HashSetDomain::new(
                    [red.clone(), green.clone(), blue.clone()]
                        .iter()
                        .cloned()
                        .collect(),
                )) as Box<dyn DomainRepresentation<_>>,
            )
        })
        .collect::<HashMap<_, _>>();

    let semantics = Arc::new(MapColouringSemantics);
    let initial_solution = Solution {
        domains,
        semantics: semantics.clone(),
    };

    let constraints = vec![
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(wa, nt)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(wa, sa)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(nt, sa)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(nt, q)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(sa, q)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(sa, nsw)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(sa, v)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(q, nsw)),
        MapColouringConstraint::NotEqual(NotEqualConstraint::new(nsw, v)),
    ];

    (initial_solution, constraints)
}

pub fn main() {
    tracing_subscriber::fmt::init();
    println!("Solving the map colouring problem...");

    let (initial_solution, constraints) = create_problem();
    let semantics = Arc::new(MapColouringSemantics);
    let built_constraints: Vec<_> = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect();

    let solver = SolverEngine::new();
    let result = solver.solve(&built_constraints, initial_solution);

    match result {
        Ok(Some(solution)) => {
            println!("Solution found!");
            for (var, domain) in solution.domains.iter() {
                println!(
                    "Region {}: {:?}",
                    var,
                    domain.get_singleton_value().unwrap()
                );
            }
        }
        Ok(None) => println!("No solution found."),
        Err(e) => eprintln!("An error occurred: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use plico::solver::semantics::DomainSemantics;

    use super::*;

    #[test]
    fn test_map_colouring_solver() {
        let (initial_solution, constraints) = create_problem();
        let semantics = Arc::new(MapColouringSemantics);
        let built_constraints: Vec<_> = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect();

        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        assert!(result.is_ok());
        let maybe_solution = result.unwrap();
        assert!(maybe_solution.is_some());

        let solution = maybe_solution.unwrap();
        // Check that all variables have a single value.
        for domain in solution.domains.values() {
            assert!(domain.is_singleton());
        }

        // Check a couple of constraints manually.
        let wa_colour = solution.domains.get(&0).unwrap().get_singleton_value();
        let nt_colour = solution.domains.get(&1).unwrap().get_singleton_value();
        assert_ne!(wa_colour, nt_colour);

        let sa_colour = solution.domains.get(&2).unwrap().get_singleton_value();
        assert_ne!(wa_colour, sa_colour);
        assert_ne!(nt_colour, sa_colour);
    }

    #[cfg(test)]
    mod prop_tests {
        use std::collections::HashSet;

        use proptest::prelude::*;

        use super::*;

        fn generate_map_colouring_problem() -> impl Strategy<
            Value = (
                usize,
                Vec<(VariableId, VariableId)>,
                usize,
                Solution<MapColouringSemantics>,
            ),
        > {
            (2..15usize)
                .prop_flat_map(|num_regions| {
                    (
                        Just(num_regions),
                        proptest::collection::vec(
                            (0..num_regions as u32, 0..num_regions as u32)
                                .prop_filter("edges must be between different regions", |(a, b)| {
                                    a != b
                                })
                                .prop_map(|(a, b)| if a < b { (a, b) } else { (b, a) }),
                            0..=(num_regions * (num_regions - 1) / 2).min(30),
                        )
                        .prop_map(|edges| {
                            let unique_edges: HashSet<(u32, u32)> = edges.into_iter().collect();
                            unique_edges.into_iter().collect::<Vec<_>>()
                        }),
                        2..5usize, // Number of colors
                    )
                })
                .prop_map(|(num_regions, adjacencies, num_colours)| {
                    let colours: Vec<_> = (0..num_colours)
                        .map(|i| MapColouringValue::Std(StandardValue::Int(i as i64)))
                        .collect();

                    let domains = (0..num_regions as u32)
                        .map(|id| {
                            (
                                id,
                                Box::new(HashSetDomain::new(colours.iter().cloned().collect()))
                                    as Box<dyn DomainRepresentation<_>>,
                            )
                        })
                        .collect::<HashMap<_, _>>();

                    let semantics = Arc::new(MapColouringSemantics);
                    let initial_solution = Solution {
                        domains,
                        semantics: semantics.clone(),
                    };

                    (num_regions, adjacencies, num_colours, initial_solution)
                })
        }

        proptest! {
            #[test]
            fn can_solve_random_maps(
                (_num_regions, adjacencies, _num_colours, initial_solution) in generate_map_colouring_problem()
            ) {
                let constraints: Vec<_> = adjacencies
                    .iter()
                    .map(|(a, b)| MapColouringConstraint::NotEqual(NotEqualConstraint::new(*a, *b)))
                    .collect();

                let semantics = Arc::new(MapColouringSemantics);
                let built_constraints: Vec<_> = constraints.iter()
                    .map(|c| semantics.build_constraint(c))
                    .collect();

                let solver = SolverEngine::new();
                let result = solver.solve(&built_constraints, initial_solution);

                assert!(result.is_ok());

                if let Some(solution) = result.unwrap() {
                    // If a solution is found, it must be valid.
                    for (u, v) in adjacencies {
                        let colour_u = solution.domains.get(&u).unwrap().get_singleton_value();
                        let colour_v = solution.domains.get(&v).unwrap().get_singleton_value();

                        prop_assert!(colour_u.is_some(), "Region {} should be coloured", u);
                        prop_assert!(colour_v.is_some(), "Region {} should be coloured", v);
                        prop_assert_ne!(colour_u, colour_v, "Adjacent regions {} and {} have the same colour", u, v);
                    }
                }
                // If no solution is found, that's okay too. We don't assert anything.
            }
        }
    }
}
