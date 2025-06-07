use crate::solver::{
    constraint::Constraint, constraints::not_equal::NotEqualConstraint, semantics::DomainSemantics,
};

#[derive(Debug)]
pub struct MapColouringSemantics;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Colour {
    Red,
    Green,
    Blue,
    Yellow,
}

impl DomainSemantics for MapColouringSemantics {
    type Value = Colour;
    type ConstraintDefinition = NotEqualConstraint<Self>;

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        Box::new(NotEqualConstraint::new(def.vars[0], def.vars[1]))
    }
}

#[cfg(test)]
mod tests {
    use super::{Colour, MapColouringSemantics};
    use crate::solver::{
        constraints::not_equal::NotEqualConstraint,
        engine::SolverEngine,
        semantics::DomainSemantics,
        solution::{CandidateSolution, DomainRepresentation, HashSetDomain},
    };
    use im::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_map_colouring() {
        let _ = tracing_subscriber::fmt::try_init();

        let wa = 0;
        let nt = 1;
        let sa = 2;
        let q = 3;

        let variables = vec![wa, nt, sa, q];
        let mut domains = HashMap::new();
        let colours: im::HashSet<Colour> = [Colour::Red, Colour::Green, Colour::Blue]
            .iter()
            .cloned()
            .collect();

        for var in &variables {
            let domain: Box<dyn DomainRepresentation<Colour>> =
                Box::new(HashSetDomain::new(colours.clone()));
            domains.insert(*var, domain);
        }

        let semantics = Arc::new(MapColouringSemantics);
        let initial_solution = CandidateSolution {
            domains,
            semantics: semantics.clone(),
        };

        let constraints = [
            NotEqualConstraint::new(wa, nt),
            NotEqualConstraint::new(wa, sa),
            NotEqualConstraint::new(nt, sa),
            NotEqualConstraint::new(nt, q),
            NotEqualConstraint::new(sa, q),
        ];

        let built_constraints: Vec<_> = constraints
            .iter()
            .map(|c| semantics.build_constraint(c))
            .collect();

        let solver = SolverEngine::new();
        let result = solver.solve(&built_constraints, initial_solution);

        assert!(result.is_ok());
        let solution = result.unwrap().unwrap();

        // Check that all variables are assigned a single colour
        for var in &variables {
            assert!(solution.domains.get(var).unwrap().is_singleton());
        }

        // Check that adjacent regions have different colours
        let wa_colour = solution.domains.get(&wa).unwrap().get_singleton_value();
        let nt_colour = solution.domains.get(&nt).unwrap().get_singleton_value();
        let sa_colour = solution.domains.get(&sa).unwrap().get_singleton_value();
        let q_colour = solution.domains.get(&q).unwrap().get_singleton_value();

        assert_ne!(wa_colour, nt_colour);
        assert_ne!(wa_colour, sa_colour);
        assert_ne!(nt_colour, sa_colour);
        assert_ne!(nt_colour, q_colour);
        assert_ne!(sa_colour, q_colour);
    }

    #[cfg(test)]
    mod prop_tests {
        use super::{Colour, MapColouringSemantics};
        use crate::solver::{
            constraints::not_equal::NotEqualConstraint,
            engine::{SolverEngine, VariableId},
            semantics::DomainSemantics,
            solution::{CandidateSolution, DomainRepresentation, HashSetDomain},
        };
        use im::HashMap;
        use proptest::prelude::*;
        use std::collections::HashSet;
        use std::sync::Arc;

        fn generate_map_colouring_problem() -> impl Strategy<Value = (usize, Vec<(u32, u32)>)> {
            (2..15usize).prop_flat_map(|num_regions| {
                let edges_strategy = proptest::collection::vec(
                    (0..num_regions as u32, 0..num_regions as u32)
                        .prop_filter("edges must be between different regions", |(a, b)| a != b)
                        .prop_map(|(a, b)| if a < b { (a, b) } else { (b, a) }),
                    0..=(num_regions * (num_regions - 1) / 2).min(30),
                )
                .prop_map(|edges| {
                    let unique_edges: HashSet<(u32, u32)> = edges.into_iter().collect();
                    unique_edges.into_iter().collect::<Vec<_>>()
                });

                (Just(num_regions), edges_strategy)
            })
        }

        proptest! {
            #[test]
            fn can_solve_random_maps((num_regions, adjacencies) in generate_map_colouring_problem()) {
                let variables: Vec<VariableId> = (0..num_regions as u32).collect();

                let mut domains = HashMap::new();
                let colours: im::HashSet<Colour> =
                    [Colour::Red, Colour::Green, Colour::Blue, Colour::Yellow].iter().cloned().collect();

                prop_assume!(colours.len() > 1);

                for &var_id in &variables {
                    let domain: Box<dyn DomainRepresentation<Colour>> =
                        Box::new(HashSetDomain::new(colours.clone()));
                    domains.insert(var_id, domain);
                }

                let semantics = Arc::new(MapColouringSemantics);
                let initial_solution = CandidateSolution { domains, semantics: semantics.clone() };

                let constraints: Vec<_> = adjacencies.iter()
                    .map(|(a, b)| NotEqualConstraint::new(*a, *b))
                    .collect();

                let built_constraints: Vec<_> = constraints.iter()
                    .map(|c| semantics.build_constraint(c))
                    .collect();

                let solver = SolverEngine::new();
                let result = solver.solve(&built_constraints, initial_solution);

                assert!(result.is_ok());

                if let Some(solution) = result.unwrap() {
                    for (u, v) in adjacencies {
                        let colour_u = solution.domains.get(&u).unwrap().get_singleton_value();
                        let colour_v = solution.domains.get(&v).unwrap().get_singleton_value();

                        prop_assert!(colour_u.is_some(), "Region {} should be coloured", u);
                        prop_assert!(colour_v.is_some(), "Region {} should be coloured", v);
                        prop_assert_ne!(colour_u, colour_v, "Adjacent regions {} and {} have the same colour", u, v);
                    }
                }
            }
        }
    }
}
