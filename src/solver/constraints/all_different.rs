use im::HashSet;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
    },
};

/// A constraint that ensures all variables in a given set have unique values.
///
/// This is a common global constraint used in problems like Sudoku. This
/// implementation achieves consistency by waiting for a variable in the set to
/// become a singleton, and then pruning that singleton's value from the domains
/// of all other variables in the set. More advanced propagation algorithms exist,
/// but this one is simple and effective.
#[derive(Debug, Clone)]
pub struct AllDifferentConstraint<S: DomainSemantics + std::fmt::Debug> {
    pub vars: Vec<VariableId>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> AllDifferentConstraint<S> {
    /// Creates a new `AllDifferentConstraint` over the given set of variables.
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

    fn descriptor(&self) -> ConstraintDescriptor {
        let vars_str = self
            .vars
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        ConstraintDescriptor {
            name: "AllDifferentConstraint".to_string(),
            description: format!("AllDifferent({})", vars_str),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
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
                let new_solution = solution.clone_with_domains(new_domains);
                return Ok(Some(new_solution));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use im::{HashMap, HashSet};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::solver::{
        solution::{DomainRepresentation, HashSetDomain},
        value::StandardValue,
    };

    // --- Test Setup ---

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct TestValue(StandardValue);

    impl From<StandardValue> for TestValue {
        fn from(v: StandardValue) -> Self {
            Self(v)
        }
    }

    #[derive(Debug, Clone)]
    struct TestSemantics;

    impl DomainSemantics for TestSemantics {
        type Value = TestValue;
        type ConstraintDefinition = ();
        type VariableMetadata = ();

        fn build_constraint(
            &self,
            _definition: &Self::ConstraintDefinition,
        ) -> Box<dyn Constraint<Self>> {
            unimplemented!("Not needed for constraint unit tests")
        }
    }

    fn int_val(i: i64) -> TestValue {
        TestValue(StandardValue::Int(i))
    }

    // --- Tests ---

    #[test]
    fn revise_prunes_singleton_value_from_peers() {
        let v1: VariableId = 0;
        let v2: VariableId = 1;
        let v3: VariableId = 2;
        let constraint = AllDifferentConstraint::<TestSemantics>::new(vec![v1, v2, v3]);

        let domains = im::hashmap! {
            v1 => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            v2 => Box::new(HashSetDomain::new(
                    [int_val(1)].into_iter().collect() // v2 is a singleton
                )) as Box<dyn DomainRepresentation<TestValue>>,
            v3 => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(3)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        // Revise v1, it should have v2's value (1) removed.
        let new_solution = constraint.revise(&v1, &solution).unwrap().unwrap();
        let new_v1_domain = new_solution.domains.get(&v1).unwrap();
        let expected_v1_domain: HashSet<TestValue> = [int_val(2)].into_iter().collect();

        assert_eq!(
            new_v1_domain.iter().cloned().collect::<HashSet<_>>(),
            expected_v1_domain
        );
        assert!(new_v1_domain.is_singleton());
    }

    #[test]
    fn revise_does_nothing_if_no_singletons() {
        let v1: VariableId = 0;
        let v2: VariableId = 1;
        let constraint = AllDifferentConstraint::<TestSemantics>::new(vec![v1, v2]);

        let domains = im::hashmap! {
            v1 => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            v2 => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&v1, &solution).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn revise_works_with_multiple_singletons_to_prune() {
        let v1: VariableId = 0;
        let v2: VariableId = 1;
        let v3: VariableId = 2;
        let constraint = AllDifferentConstraint::<TestSemantics>::new(vec![v1, v2, v3]);

        let domains = im::hashmap! {
            v1 => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2), int_val(3)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            v2 => Box::new(HashSetDomain::new(
                    [int_val(1)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            v3 => Box::new(HashSetDomain::new(
                    [int_val(2)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&v1, &solution).unwrap().unwrap();
        let new_v1_domain = new_solution.domains.get(&v1).unwrap();
        let expected_v1_domain: HashSet<TestValue> = [int_val(3)].into_iter().collect();

        assert_eq!(
            new_v1_domain.iter().cloned().collect::<HashSet<_>>(),
            expected_v1_domain
        );
        assert!(new_v1_domain.is_singleton());
    }
}
