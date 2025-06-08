use std::marker::PhantomData;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
    },
};

/// A constraint that enforces inequality between two variables (`A != B`).
///
/// This constraint will prune a value from a variable's domain if the other
/// variable's domain has been reduced to a singleton containing that value.
#[derive(Debug, Clone)]
pub struct NotEqualConstraint<S: DomainSemantics> {
    /// The two variables that are constrained to not be equal.
    pub vars: [VariableId; 2],
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics> NotEqualConstraint<S> {
    /// Creates a new `NotEqualConstraint` that enforces `?a != ?b`.
    pub fn new(a: VariableId, b: VariableId) -> Self {
        Self {
            vars: [a, b],
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for NotEqualConstraint<S> {
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        ConstraintDescriptor {
            name: "NotEqualConstraint".to_string(),
            description: format!("?{} != ?{}", self.vars[0], self.vars[1]),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let other_var = if *target_var == self.vars[0] {
            self.vars[1]
        } else {
            self.vars[0]
        };

        let other_domain = solution.domains.get(&other_var).unwrap();

        // If the other variable's domain is not a singleton, we can't prune.
        if !other_domain.is_singleton() {
            return Ok(None);
        }

        // The value to remove is the single value in the other domain.
        let value_to_remove = other_domain.get_singleton_value().unwrap();

        let target_domain = solution.domains.get(target_var).unwrap();
        let original_size = target_domain.len();

        let new_domain = target_domain.retain(&|val| *val != value_to_remove);

        if new_domain.len() < original_size {
            let new_domains = solution.domains.update(*target_var, new_domain);
            let new_solution = solution.clone_with_domains(new_domains);
            Ok(Some(new_solution))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use im::HashMap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::solver::{
        solution::{DomainRepresentation, HashSetDomain},
        value::StandardValue,
    };

    // --- Test Setup ---

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    fn revise_removes_value_when_other_var_is_singleton() {
        let a: VariableId = 0;
        let b: VariableId = 1;
        let constraint = NotEqualConstraint::<TestSemantics>::new(a, b);

        let domains = im::hashmap! {
            a => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2), int_val(3)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            b => Box::new(HashSetDomain::new(
                    [int_val(2)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&a, &solution).unwrap().unwrap();
        let new_a_domain = new_solution.domains.get(&a).unwrap();
        let expected_domain: im::HashSet<TestValue> =
            [int_val(1), int_val(3)].into_iter().collect();

        assert_eq!(new_a_domain.len(), 2);
        assert_eq!(
            new_a_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_domain
        );
    }

    #[test]
    fn revise_does_nothing_when_other_var_is_not_singleton() {
        let a: VariableId = 0;
        let b: VariableId = 1;
        let constraint = NotEqualConstraint::<TestSemantics>::new(a, b);

        let domains = im::hashmap! {
            a => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2), int_val(3)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
            b => Box::new(HashSetDomain::new(
                    [int_val(2), int_val(4)].into_iter().collect()
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&a, &solution).unwrap();
        assert!(result.is_none());
    }
}
